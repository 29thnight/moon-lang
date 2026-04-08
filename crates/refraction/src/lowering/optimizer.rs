//! Optimizer Enhancement passes (v4 Section 23, `opt.v4`).
//!
//! Operates on the C# IR after lowering. The current passes are deliberately
//! conservative — they only rewrite shapes the lowering produces verbatim and
//! never alter program semantics. The first cut focuses on the three rules
//! defined in spec §23.2:
//!
//! 1. **String Interpolation Caching** (`opt.string`) — repeated identical
//!    `$"..."` expressions in `Update`/`FixedUpdate` get a cached field +
//!    last-state field, so the allocation only happens when the inputs change.
//! 2. **LINQ Elimination** (`opt.linq`) — `xs.Where(...).ToList()` and
//!    `xs.Select(...).ToList()` chains in hot paths get rewritten to a manual
//!    `for` loop variant.
//! 3. **Struct Copy Reduction** (`opt.structcopy`) — for hot paths, repeated
//!    reads of a known-large struct local (`Vector3`/`Quaternion`/`Matrix4x4`)
//!    are tagged with `ref readonly` markers (currently emitted as comments
//!    until ref locals are wired through the IR).
//!
//! The optimizer also surfaces hot-path warnings W026/W027 even when no
//! rewrite was applied — this lets users opt out of automatic transformations
//! while still seeing the lint.

use super::csharp_ir::*;
use crate::diagnostics::Diagnostic;
use crate::lexer::token::{Position, Span};

/// Configuration for the optimizer pass.
#[derive(Debug, Clone, Copy)]
pub struct OptimizerOptions {
    pub enabled: bool,
    pub cache_string_interp: bool,
    pub eliminate_linq: bool,
    pub reduce_struct_copies: bool,
}

impl Default for OptimizerOptions {
    fn default() -> Self {
        OptimizerOptions {
            enabled: true,
            cache_string_interp: true,
            eliminate_linq: true,
            reduce_struct_copies: true,
        }
    }
}

/// Result of running the optimizer.
#[derive(Debug, Default, Clone)]
pub struct OptimizerReport {
    pub strings_cached: u32,
    pub linq_chains_rewritten: u32,
    pub struct_copies_reduced: u32,
    pub diagnostics: Vec<Diagnostic>,
}

impl OptimizerReport {
    pub fn is_empty(&self) -> bool {
        self.strings_cached == 0
            && self.linq_chains_rewritten == 0
            && self.struct_copies_reduced == 0
            && self.diagnostics.is_empty()
    }
}

/// Run the optimizer over a C# IR file in place.
pub fn optimize(file: &mut CsFile, options: OptimizerOptions) -> OptimizerReport {
    let mut report = OptimizerReport::default();
    if !options.enabled {
        return report;
    }
    optimize_class(&mut file.class, options, &mut report);
    for extra in &mut file.extra_types {
        optimize_class(extra, options, &mut report);
    }
    report
}

fn optimize_class(class: &mut CsClass, options: OptimizerOptions, report: &mut OptimizerReport) {
    // Two-phase walk: first collect new fields and rewrites, then apply.
    let mut new_fields: Vec<CsMember> = Vec::new();

    let class_name = class.name.clone();
    for member in class.members.iter_mut() {
        if let CsMember::Method {
            name,
            body,
            source_span,
            ..
        } = member
        {
            let in_hot_path = is_hot_path_method(name);
            let span = source_span.unwrap_or(default_span());
            let mut hot_ctx = HotPathContext {
                in_hot_path,
                method_name: name.clone(),
                class_name: class_name.clone(),
                span,
                report,
                options,
                new_fields: &mut new_fields,
                linq_counter: 0,
            };
            optimize_block(body, &mut hot_ctx);
        }
    }

    // Insert generated cache fields at the top of the class so they appear
    // before the methods that use them — keeps the generated C# readable.
    if !new_fields.is_empty() {
        let mut combined = Vec::with_capacity(class.members.len() + new_fields.len());
        combined.extend(new_fields);
        combined.extend(std::mem::take(&mut class.members));
        class.members = combined;
    }
}

struct HotPathContext<'a> {
    in_hot_path: bool,
    method_name: String,
    #[allow(dead_code)]
    class_name: String,
    span: Span,
    report: &'a mut OptimizerReport,
    options: OptimizerOptions,
    new_fields: &'a mut Vec<CsMember>,
    /// Issue #92: monotonic counter for LINQ rewrite loop variables
    /// so two rewrites in the same method do not collide on a shared
    /// `__opt_i` local name.
    linq_counter: u32,
}

fn optimize_block(stmts: &mut Vec<CsStmt>, ctx: &mut HotPathContext) {
    let mut i = 0;
    while i < stmts.len() {
        if optimize_stmt_at(stmts, i, ctx) {
            // Statement was replaced/expanded — re-check the same index.
            // The optimizer never produces infinite loops because each pass
            // strictly reduces or rewrites in a known finite shape.
            continue;
        }
        i += 1;
    }
}

/// Returns true if the statement was rewritten and the caller should not
/// advance — used when a single statement is expanded into multiple.
fn optimize_stmt_at(stmts: &mut Vec<CsStmt>, idx: usize, ctx: &mut HotPathContext) -> bool {
    let stmt = &mut stmts[idx];
    match stmt {
        CsStmt::Block(inner, _) => {
            optimize_block(inner, ctx);
        }
        CsStmt::If { then_body, else_body, .. } => {
            optimize_block(then_body, ctx);
            if let Some(else_body) = else_body {
                optimize_block(else_body, ctx);
            }
        }
        CsStmt::For { body, .. } => optimize_block(body, ctx),
        CsStmt::ForEach { body, .. } => optimize_block(body, ctx),
        CsStmt::While { body, .. } => optimize_block(body, ctx),
        CsStmt::UseBlock { body, .. } => optimize_block(body, ctx),
        CsStmt::Switch { cases, .. } => {
            for case in cases.iter_mut() {
                optimize_block(&mut case.body, ctx);
            }
        }
        CsStmt::TryCatch { try_body, catches, finally_body, .. } => {
            optimize_block(try_body, ctx);
            for c in catches {
                optimize_block(&mut c.body, ctx);
            }
            if let Some(fb) = finally_body {
                optimize_block(fb, ctx);
            }
        }
        _ => {}
    }

    if !ctx.in_hot_path {
        return false;
    }

    // Per-statement rewrites that depend on hot-path context.
    if ctx.options.cache_string_interp {
        if let Some(rewrite) = try_cache_string_interp(stmts, idx, ctx) {
            apply_string_cache_rewrite(stmts, idx, rewrite, ctx);
            return true;
        }
    }

    if ctx.options.eliminate_linq {
        if let Some(rewrite) = try_eliminate_linq(stmts, idx, ctx) {
            apply_linq_rewrite(stmts, idx, rewrite, ctx);
            return true;
        }
    }

    if ctx.options.reduce_struct_copies {
        try_reduce_struct_copy(stmts, idx, ctx);
    }

    false
}

// === Hot path detection ===

/// Methods that run every frame and therefore benefit from optimization.
fn is_hot_path_method(name: &str) -> bool {
    matches!(name, "Update" | "FixedUpdate" | "LateUpdate")
}

fn default_span() -> Span {
    Span {
        start: Position { line: 1, col: 1 },
        end: Position { line: 1, col: 1 },
    }
}

// === Rule 1: String Interpolation Caching (opt.string) ===

#[derive(Debug, Clone)]
struct StringCacheRewrite {
    /// The C# `target` of an assignment such as `label.text`.
    target: String,
    /// The original `$"..."` text — kept verbatim so we don't break formatting.
    interp_text: String,
    /// Argument identifiers detected inside the interpolation.
    captured_args: Vec<String>,
    /// Generated unique suffix derived from `target`.
    cache_suffix: String,
}

/// Match a single statement of the form
/// `target = $"...{argA}...{argB}...";` and return the rewrite plan.
fn try_cache_string_interp(
    stmts: &[CsStmt],
    idx: usize,
    _ctx: &mut HotPathContext,
) -> Option<StringCacheRewrite> {
    let CsStmt::Assignment { target, op, value, .. } = &stmts[idx] else {
        return None;
    };
    if op != "=" {
        return None;
    }
    // Skip rewrites of statements we generated ourselves to prevent infinite
    // expansion when the optimizer re-walks a stmt slice that already contains
    // a cached field assignment.
    if target.starts_with("__opt_cached_") || target.starts_with("__opt_prev_") {
        return None;
    }
    if !value.trim_start().starts_with("$\"") || !value.trim_end().ends_with('"') {
        return None;
    }

    let captured = extract_interp_identifiers(value);
    if captured.is_empty() {
        return None;
    }

    Some(StringCacheRewrite {
        target: target.clone(),
        interp_text: value.clone(),
        captured_args: captured,
        cache_suffix: sanitize_for_field(target),
    })
}

fn apply_string_cache_rewrite(
    stmts: &mut Vec<CsStmt>,
    idx: usize,
    rewrite: StringCacheRewrite,
    ctx: &mut HotPathContext,
) {
    // Generate fields: prev_<args>, cached_<suffix>
    let cached_field = format!("__opt_cached_{}", rewrite.cache_suffix);
    let mut prev_field_names: Vec<String> = Vec::new();
    for arg in &rewrite.captured_args {
        let prev = format!("__opt_prev_{}_{}", rewrite.cache_suffix, sanitize_for_field(arg));
        ctx.new_fields.push(CsMember::Field {
            attributes: vec![],
            modifiers: "private".into(),
            ty: "object".into(),
            name: prev.clone(),
            init: None,
        });
        prev_field_names.push(prev);
    }
    ctx.new_fields.push(CsMember::Field {
        attributes: vec![],
        modifiers: "private".into(),
        ty: "string".into(),
        name: cached_field.clone(),
        init: None,
    });

    // Build the if-condition: any input changed?
    let cond_parts: Vec<String> = rewrite
        .captured_args
        .iter()
        .zip(prev_field_names.iter())
        .map(|(arg, prev)| format!("!object.Equals({}, {})", prev, arg))
        .collect();
    let cond_text = cond_parts.join(" || ");

    let mut then_body: Vec<CsStmt> = Vec::new();
    for (arg, prev) in rewrite.captured_args.iter().zip(prev_field_names.iter()) {
        then_body.push(CsStmt::Assignment {
            target: prev.clone(),
            op: "=".into(),
            value: arg.clone(),
            source_span: Some(ctx.span),
        });
    }
    then_body.push(CsStmt::Assignment {
        target: cached_field.clone(),
        op: "=".into(),
        value: rewrite.interp_text.clone(),
        source_span: Some(ctx.span),
    });

    let if_stmt = CsStmt::If {
        cond: cond_text,
        then_body,
        else_body: None,
        source_span: Some(ctx.span),
    };
    let final_assign = CsStmt::Assignment {
        target: rewrite.target.clone(),
        op: "=".into(),
        value: cached_field.clone(),
        source_span: Some(ctx.span),
    };

    // Replace the original assignment with [if, final_assign].
    stmts.splice(idx..=idx, [if_stmt, final_assign]);

    ctx.report.strings_cached += 1;
    ctx.report.diagnostics.push(
        Diagnostic::warning(
            "W026",
            format!(
                "String allocation cached on hot path '{}' (target '{}')",
                ctx.method_name, rewrite.target
            ),
            ctx.span,
        )
        .with_help("Optimizer applied opt.string: cached interpolation in a field"),
    );
}

/// Pull `arg` identifiers out of a `$"...{arg}..."` literal in a best-effort
/// way. Only single bare identifiers are captured — anything more complex
/// (`{a + b}`) is left untouched and the caching pass bails out by returning
/// the empty set.
fn extract_interp_identifiers(value: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j >= bytes.len() {
                return Vec::new();
            }
            let inner = &value[start..j];
            if !is_simple_ident(inner.trim()) {
                return Vec::new();
            }
            let id = inner.trim().to_string();
            if !out.contains(&id) {
                out.push(id);
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

fn is_simple_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false)
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn sanitize_for_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

// === Rule 2: LINQ Elimination (opt.linq) ===

#[derive(Debug, Clone)]
struct LinqRewrite {
    target_var: String,
    declared_ty: String,
    source_collection: String,
    operation: LinqOp,
}

#[derive(Debug, Clone)]
enum LinqOp {
    /// `xs.Where(<lambda>).ToList()`
    Where { predicate: String },
    /// `xs.Select(<lambda>).ToList()` — `target_elem_ty` is reserved for the
    /// next iteration when explicit element types are inferred from the
    /// surrounding declaration.
    Select {
        projection: String,
        #[allow(dead_code)]
        target_elem_ty: String,
    },
}

fn try_eliminate_linq(
    stmts: &[CsStmt],
    idx: usize,
    _ctx: &mut HotPathContext,
) -> Option<LinqRewrite> {
    let CsStmt::VarDecl { ty, name, init, .. } = &stmts[idx] else {
        return None;
    };
    let init_trim = init.trim();
    // Pattern A: Where(...).ToList()
    if let Some((source, predicate)) = parse_where_to_list(init_trim) {
        return Some(LinqRewrite {
            target_var: name.clone(),
            declared_ty: ty.clone(),
            source_collection: source,
            operation: LinqOp::Where { predicate },
        });
    }
    // Pattern B: Select(...).ToList()
    if let Some((source, projection)) = parse_select_to_list(init_trim) {
        return Some(LinqRewrite {
            target_var: name.clone(),
            declared_ty: ty.clone(),
            source_collection: source,
            operation: LinqOp::Select {
                projection,
                target_elem_ty: "var".into(),
            },
        });
    }
    None
}

fn parse_where_to_list(expr: &str) -> Option<(String, String)> {
    // Best-effort: source.Where(<predicate>).ToList()
    let suffix = ".ToList()";
    if !expr.ends_with(suffix) {
        return None;
    }
    let head = &expr[..expr.len() - suffix.len()];
    let close_paren = head.rfind(')')?;
    if close_paren + 1 != head.len() {
        return None;
    }
    let where_token = ".Where(";
    let where_idx = head.rfind(where_token)?;
    if !head[..where_idx].chars().any(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let source = &head[..where_idx];
    let predicate_start = where_idx + where_token.len();
    let predicate_end = matched_paren_end(head, where_idx + where_token.len() - 1)?;
    if predicate_end + 1 != head.len() {
        return None;
    }
    let predicate = &head[predicate_start..predicate_end];
    Some((source.to_string(), predicate.to_string()))
}

fn parse_select_to_list(expr: &str) -> Option<(String, String)> {
    let suffix = ".ToList()";
    if !expr.ends_with(suffix) {
        return None;
    }
    let head = &expr[..expr.len() - suffix.len()];
    let select_token = ".Select(";
    let select_idx = head.rfind(select_token)?;
    let source = &head[..select_idx];
    let projection_start = select_idx + select_token.len();
    let projection_end = matched_paren_end(head, select_idx + select_token.len() - 1)?;
    if projection_end + 1 != head.len() {
        return None;
    }
    let projection = &head[projection_start..projection_end];
    Some((source.to_string(), projection.to_string()))
}

/// Issue #97: extract `T` from a declared C# type string like
/// `System.Collections.Generic.List<int>` or `List<int>`. Returns
/// `"object"` as a conservative fallback when the declared type is
/// not a recognizable list-like generic.
fn extract_list_element_type(declared_ty: &str) -> String {
    let trimmed = declared_ty.trim();
    // Try the fully-qualified form first, then the short form.
    for prefix in [
        "System.Collections.Generic.List<",
        "List<",
        "System.Collections.Generic.IList<",
        "IList<",
        "System.Collections.Generic.IEnumerable<",
        "IEnumerable<",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if let Some(inner) = rest.strip_suffix('>') {
                // Guard against nested generics carrying trailing whitespace.
                return inner.trim().to_string();
            }
        }
    }
    "object".into()
}

fn matched_paren_end(s: &str, open: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(open)? != &b'(' {
        return None;
    }
    let mut depth = 0;
    for (i, b) in bytes.iter().enumerate().skip(open) {
        if *b == b'(' {
            depth += 1;
        } else if *b == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn apply_linq_rewrite(
    stmts: &mut Vec<CsStmt>,
    idx: usize,
    rewrite: LinqRewrite,
    ctx: &mut HotPathContext,
) {
    let span = ctx.span;
    let LinqRewrite {
        target_var,
        declared_ty,
        source_collection,
        operation,
    } = rewrite;

    let mut new_stmts: Vec<CsStmt> = Vec::new();
    let final_ty = if declared_ty == "var" {
        // Match the source collection's natural type by reusing it as a hint.
        format!("var")
    } else {
        declared_ty.clone()
    };

    // Issue #97: infer the element type from the declared target type
    // so `val xs: List<Int> = ...Where().ToList()` rewrites to
    // `new List<int>()` instead of `new List<object>()`, avoiding the
    // boxing allocation the optimizer was supposed to eliminate.
    // Fall back to `object` only when the declared type isn't a
    // recognizable `List<T>` shape.
    let element_ty = extract_list_element_type(&declared_ty);
    let temp_init = format!(
        "new System.Collections.Generic.List<{}>()",
        element_ty
    );

    new_stmts.push(CsStmt::VarDecl {
        ty: final_ty,
        name: target_var.clone(),
        init: temp_init,
        source_span: Some(span),
    });
    // Issue #92: use a unique counter name per LINQ rewrite so two
    // rewrites in the same method don't collide on `__opt_i`.
    let counter = format!("__opt_i_{}", ctx.linq_counter);
    ctx.linq_counter += 1;
    let body: Vec<CsStmt> = match &operation {
        LinqOp::Where { predicate } => vec![CsStmt::If {
            cond: invoke_lambda(predicate, &format!("{}[{}]", source_collection, counter)),
            then_body: vec![CsStmt::Expr(
                format!("{}.Add({}[{}])", target_var, source_collection, counter),
                Some(span),
            )],
            else_body: None,
            source_span: Some(span),
        }],
        LinqOp::Select { projection, .. } => vec![CsStmt::Expr(
            format!(
                "{}.Add({})",
                target_var,
                invoke_lambda(projection, &format!("{}[{}]", source_collection, counter))
            ),
            Some(span),
        )],
    };
    new_stmts.push(CsStmt::For {
        init: format!("int {} = 0", counter),
        cond: format!("{} < {}.Count", counter, source_collection),
        incr: format!("{}++", counter),
        body,
        source_span: Some(span),
    });

    stmts.splice(idx..=idx, new_stmts);
    ctx.report.linq_chains_rewritten += 1;
    ctx.report.diagnostics.push(
        Diagnostic::warning(
            "W027",
            format!(
                "LINQ allocation in hot path '{}' rewritten as for loop",
                ctx.method_name
            ),
            span,
        )
        .with_help("Optimizer applied opt.linq: replaced LINQ chain with manual loop"),
    );
}

/// Substitute the lambda parameter with the iteration item — a tiny rewriter
/// that handles `e => expr` and `(e) => expr` shapes.
fn invoke_lambda(lambda: &str, item: &str) -> String {
    let trimmed = lambda.trim();
    if let Some(arrow_idx) = trimmed.find("=>") {
        let (params, body) = trimmed.split_at(arrow_idx);
        let body = body[2..].trim();
        let param_name = params.trim().trim_start_matches('(').trim_end_matches(')').trim();
        if is_simple_ident(param_name) {
            // Replace whole-word param occurrences in body.
            return replace_word(body, param_name, item);
        }
    }
    format!("({})({})", lambda, item)
}

fn replace_word(input: &str, word: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last = 0;
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if input[i..].starts_with(word) {
            let prev_ok = i == 0
                || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            let next_pos = i + word.len();
            let next_ok = next_pos == bytes.len()
                || !(bytes[next_pos].is_ascii_alphanumeric() || bytes[next_pos] == b'_');
            if prev_ok && next_ok {
                out.push_str(&input[last..i]);
                out.push_str(replacement);
                i = next_pos;
                last = i;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&input[last..]);
    out
}

// === Rule 3: Struct Copy Reduction (opt.structcopy) ===

/// Conservative struct-copy elimination: when a hot-path method declares
/// `var v = expr;` and the type clearly comes from a known-large value type,
/// emit a comment hint near the declaration. We don't change semantics — we
/// only mark the site so downstream lowering can later opt into `ref readonly`.
///
/// Returns true when a hint comment was inserted (caller advances past it).
fn try_reduce_struct_copy(stmts: &mut Vec<CsStmt>, idx: usize, ctx: &mut HotPathContext) -> bool {
    let CsStmt::VarDecl { ty, name, init, .. } = &stmts[idx] else {
        return false;
    };
    let init_trim = init.trim();
    let large_struct_hint = init_trim.starts_with("transform.")
        || init_trim.starts_with("transform.position")
        || init_trim.contains("Vector3")
        || init_trim.contains("Quaternion")
        || init_trim.contains("Matrix4x4");
    if !large_struct_hint {
        return false;
    }
    if ty != "var" && !ty.contains("Vector3") && !ty.contains("Quaternion") && !ty.contains("Matrix4x4") {
        return false;
    }
    // Avoid inserting a duplicate comment if we've already tagged this site.
    if idx > 0 {
        if let CsStmt::Raw(prev, _) = &stmts[idx - 1] {
            if prev.contains("opt.structcopy") {
                return false;
            }
        }
    }
    // Insert a comment node above the declaration to record the optimization
    // hint. This keeps the C# valid and the hint visible.
    let comment = CsStmt::Raw(
        format!(
            "// opt.structcopy: hot-path read of '{}' (consider using ref readonly when supported)",
            name
        ),
        Some(ctx.span),
    );
    stmts.insert(idx, comment);
    ctx.report.struct_copies_reduced += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_method(name: &str, body: Vec<CsStmt>) -> CsMember {
        CsMember::Method {
            attributes: vec![],
            modifiers: "public".into(),
            return_ty: "void".into(),
            name: name.into(),
            params: vec![],
            where_clauses: vec![],
            body,
            source_span: Some(default_span()),
        }
    }

    fn class_with(method: CsMember) -> CsFile {
        CsFile {
            header_comment: String::new(),
            usings: vec![],
            class: CsClass {
                attributes: vec![],
                modifiers: "public".into(),
                name: "TestClass".into(),
                base_class: None,
                interfaces: vec![],
                where_clauses: vec![],
                members: vec![method],
            },
            extra_types: vec![],
        }
    }

    #[test]
    fn caches_string_interpolation_in_update() {
        let stmt = CsStmt::Assignment {
            target: "label.text".into(),
            op: "=".into(),
            value: "$\"HP: {hp}\"".into(),
            source_span: Some(default_span()),
        };
        let method = make_method("Update", vec![stmt]);
        let mut file = class_with(method);
        let report = optimize(&mut file, OptimizerOptions::default());
        assert_eq!(report.strings_cached, 1);
        assert!(report.diagnostics.iter().any(|d| d.code == "W026"));
        // Verify a cached field was added.
        let has_cached_field = file
            .class
            .members
            .iter()
            .any(|m| matches!(m, CsMember::Field { name, .. } if name.starts_with("__opt_cached_")));
        assert!(has_cached_field, "expected cached field, got {:?}", file.class.members);
        // The original method now contains an If guarding the cache update.
        let method_body = file
            .class
            .members
            .iter()
            .find_map(|m| match m {
                CsMember::Method { name, body, .. } if name == "Update" => Some(body),
                _ => None,
            })
            .expect("Update method");
        assert!(method_body
            .iter()
            .any(|s| matches!(s, CsStmt::If { .. })));
    }

    #[test]
    fn does_not_cache_string_outside_hot_path() {
        let stmt = CsStmt::Assignment {
            target: "label.text".into(),
            op: "=".into(),
            value: "$\"HP: {hp}\"".into(),
            source_span: Some(default_span()),
        };
        let method = make_method("OnEnable", vec![stmt]);
        let mut file = class_with(method);
        let report = optimize(&mut file, OptimizerOptions::default());
        assert_eq!(report.strings_cached, 0);
    }

    #[test]
    fn eliminates_where_to_list_in_update() {
        let stmt = CsStmt::VarDecl {
            ty: "var".into(),
            name: "alive".into(),
            init: "enemies.Where(e => e.IsAlive).ToList()".into(),
            source_span: Some(default_span()),
        };
        let method = make_method("Update", vec![stmt]);
        let mut file = class_with(method);
        let report = optimize(&mut file, OptimizerOptions::default());
        assert_eq!(report.linq_chains_rewritten, 1);
        assert!(report.diagnostics.iter().any(|d| d.code == "W027"));
        let body = file
            .class
            .members
            .iter()
            .find_map(|m| match m {
                CsMember::Method { name, body, .. } if name == "Update" => Some(body),
                _ => None,
            })
            .expect("Update method");
        assert!(body.iter().any(|s| matches!(s, CsStmt::For { .. })));
    }

    #[test]
    fn eliminates_select_to_list() {
        let stmt = CsStmt::VarDecl {
            ty: "var".into(),
            name: "names".into(),
            init: "people.Select(p => p.name).ToList()".into(),
            source_span: Some(default_span()),
        };
        let method = make_method("Update", vec![stmt]);
        let mut file = class_with(method);
        let report = optimize(&mut file, OptimizerOptions::default());
        assert_eq!(report.linq_chains_rewritten, 1);
    }

    #[test]
    fn struct_copy_hint_added_for_vector3() {
        let stmt = CsStmt::VarDecl {
            ty: "var".into(),
            name: "p".into(),
            init: "transform.position".into(),
            source_span: Some(default_span()),
        };
        let method = make_method("Update", vec![stmt]);
        let mut file = class_with(method);
        let report = optimize(&mut file, OptimizerOptions::default());
        assert_eq!(report.struct_copies_reduced, 1);
        let body = file
            .class
            .members
            .iter()
            .find_map(|m| match m {
                CsMember::Method { name, body, .. } if name == "Update" => Some(body),
                _ => None,
            })
            .expect("Update method");
        assert!(body
            .iter()
            .any(|s| matches!(s, CsStmt::Raw(comment, _) if comment.contains("opt.structcopy"))));
    }

    #[test]
    fn disabled_optimizer_makes_no_changes() {
        let stmt = CsStmt::Assignment {
            target: "label.text".into(),
            op: "=".into(),
            value: "$\"HP: {hp}\"".into(),
            source_span: Some(default_span()),
        };
        let method = make_method("Update", vec![stmt]);
        let mut file = class_with(method);
        let report = optimize(
            &mut file,
            OptimizerOptions { enabled: false, ..OptimizerOptions::default() },
        );
        assert!(report.is_empty());
    }

    #[test]
    fn extract_interp_identifiers_simple_cases() {
        assert_eq!(extract_interp_identifiers("$\"HP: {hp}\""), vec!["hp"]);
        assert_eq!(extract_interp_identifiers("$\"{a}/{b}\""), vec!["a", "b"]);
        assert!(extract_interp_identifiers("$\"{a + b}\"").is_empty());
        assert!(extract_interp_identifiers("$\"plain\"").is_empty());
    }

    #[test]
    fn invoke_lambda_substitutes_simple_param() {
        assert_eq!(invoke_lambda("e => e.IsAlive", "items[i]"), "items[i].IsAlive");
        assert_eq!(invoke_lambda("(p) => p.name", "list[0]"), "list[0].name");
    }

    #[test]
    fn replace_word_does_not_match_substrings() {
        assert_eq!(replace_word("element", "e", "X"), "element");
        assert_eq!(replace_word("e + e2", "e", "X"), "X + e2");
    }

    // Issue #92: two LINQ rewrites in the same method must use
    // distinct counter names so the emitted C# does not redeclare
    // `int __opt_i` in the same scope.
    #[test]
    fn linq_rewrites_use_unique_counter_names() {
        let stmt1 = CsStmt::VarDecl {
            ty: "System.Collections.Generic.List<int>".into(),
            name: "alive".into(),
            init: "items.Where(e => e > 0).ToList()".into(),
            source_span: Some(default_span()),
        };
        let stmt2 = CsStmt::VarDecl {
            ty: "System.Collections.Generic.List<int>".into(),
            name: "doubled".into(),
            init: "people.Select(p => p * 2).ToList()".into(),
            source_span: Some(default_span()),
        };
        let method = make_method("Update", vec![stmt1, stmt2]);
        let mut file = class_with(method);
        let _report = optimize(&mut file, OptimizerOptions::default());

        let update_body = file
            .class
            .members
            .iter()
            .find_map(|m| match m {
                CsMember::Method { name, body, .. } if name == "Update" => Some(body),
                _ => None,
            })
            .expect("Update method");

        // Collect counter names from For loop init clauses.
        let mut counter_names = Vec::new();
        for stmt in update_body {
            if let CsStmt::For { init, .. } = stmt {
                counter_names.push(init.clone());
            }
        }
        // Both rewrites must yield unique `__opt_i_N` counters.
        assert_eq!(counter_names.len(), 2, "expected two for loops, got {counter_names:?}");
        assert!(counter_names[0] != counter_names[1], "counter names must differ: {counter_names:?}");
    }

    // Issue #97: LINQ rewrite should use the declared element type,
    // not `List<object>`, to avoid boxing value types.
    #[test]
    fn linq_rewrite_uses_declared_element_type() {
        assert_eq!(
            extract_list_element_type("System.Collections.Generic.List<int>"),
            "int"
        );
        assert_eq!(
            extract_list_element_type("List<float>"),
            "float"
        );
        assert_eq!(
            extract_list_element_type("SomethingUnknown"),
            "object"
        );
    }
}
