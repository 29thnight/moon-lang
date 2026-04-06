//! C# source code emitter — converts C# IR to formatted source text.

use crate::lowering::csharp_ir::*;

/// Emit a C# file as formatted source text.
pub fn emit(file: &CsFile) -> String {
    let mut out = String::new();

    // Header comment
    out.push_str(&file.header_comment);
    out.push_str("\n\n");

    // Usings
    for u in &file.usings {
        out.push_str(&format!("using {};\n", u));
    }
    if !file.usings.is_empty() {
        out.push('\n');
    }

    // Class
    emit_class(&mut out, &file.class, 0);
    for extra_type in &file.extra_types {
        out.push('\n');
        emit_class(&mut out, extra_type, 0);
    }

    out
}

fn emit_class(out: &mut String, class: &CsClass, indent: usize) {
    let pad = "    ".repeat(indent);

    // Attributes
    for attr in &class.attributes {
        out.push_str(&format!("{}{}\n", pad, attr));
    }

    // Class header
    let is_enum = class.modifiers.contains("enum");
    let is_interface = class.modifiers.contains("interface");
    let keyword = if is_enum || is_interface { "" } else { " class" };
    let mut header = format!("{}{}{} {}", pad, class.modifiers, keyword, class.name);
    if let Some(base) = &class.base_class {
        header.push_str(&format!(" : {}", base));
    }
    if !class.interfaces.is_empty() {
        if class.base_class.is_some() {
            header.push_str(&format!(", {}", class.interfaces.join(", ")));
        } else {
            header.push_str(&format!(" : {}", class.interfaces.join(", ")));
        }
    }
    // Append where clauses (e.g. "where T : Component")
    for wc in &class.where_clauses {
        header.push_str(&format!(" {}", wc));
    }
    out.push_str(&header);
    out.push('\n');
    out.push_str(&format!("{}{{\n", pad));

    // Members
    for (i, member) in class.members.iter().enumerate() {
        if i > 0 && should_add_blank_line(member, is_enum) {
            out.push('\n');
        }
        emit_member(out, member, indent + 1, is_enum);
    }

    out.push_str(&format!("{}}}\n", pad));
}

fn should_add_blank_line(member: &CsMember, is_enum: bool) -> bool {
    if is_enum { return false; }
    matches!(member, CsMember::Method { .. })
}

fn emit_member(out: &mut String, member: &CsMember, indent: usize, _is_enum: bool) {
    let pad = "    ".repeat(indent);

    match member {
        CsMember::Field { attributes, modifiers, ty, name, init } => {
            for attr in attributes {
                out.push_str(&format!("{}{}\n", pad, attr));
            }
            if let Some(init_val) = init {
                out.push_str(&format!("{}{} {} {} = {};\n", pad, modifiers, ty, name, init_val));
            } else {
                out.push_str(&format!("{}{} {} {};\n", pad, modifiers, ty, name));
            }
        }
        CsMember::Property { modifiers, ty, name, getter_expr, setter, setter_expr } => {
            match (setter, setter_expr) {
                (Some(set_mod), Some(set_expr)) => {
                    // Full property with getter + setter
                    out.push_str(&format!("{}{} {} {}\n", pad, modifiers, ty, name));
                    out.push_str(&format!("{}{{\n", pad));
                    out.push_str(&format!("{}    get => {};\n", pad, getter_expr));
                    out.push_str(&format!("{}    {} => {} = value;\n", pad, set_mod, set_expr));
                    out.push_str(&format!("{}}}\n", pad));
                }
                _ => {
                    // Expression-bodied getter only
                    out.push_str(&format!("{}{} {} {} => {};\n", pad, modifiers, ty, name, getter_expr));
                }
            }
        }
        CsMember::Method { attributes, modifiers, return_ty, name, params, where_clauses, body, .. } => {
            for attr in attributes {
                out.push_str(&format!("{}{}\n", pad, attr));
            }
            let params_str: Vec<String> = params.iter().map(|p| {
                if let Some(def) = &p.default {
                    format!("{} {} = {}", p.ty, p.name, def)
                } else {
                    format!("{} {}", p.ty, p.name)
                }
            }).collect();
            let where_suffix = if where_clauses.is_empty() {
                String::new()
            } else {
                format!(" {}", where_clauses.join(" "))
            };
            if return_ty.is_empty() {
                out.push_str(&format!("{}{} {}({}){}\n", pad, modifiers, name, params_str.join(", "), where_suffix));
            } else {
                out.push_str(&format!("{}{} {} {}({}){}\n", pad, modifiers, return_ty, name, params_str.join(", "), where_suffix));
            }
            out.push_str(&format!("{}{{\n", pad));
            for stmt in body {
                emit_stmt(out, stmt, indent + 1);
            }
            out.push_str(&format!("{}}}\n", pad));
        }
        CsMember::RawCode(code) => {
            out.push_str(&format!("{}{}\n", pad, code));
        }
    }
}

fn emit_stmt(out: &mut String, stmt: &CsStmt, indent: usize) {
    let pad = "    ".repeat(indent);

    match stmt {
        CsStmt::VarDecl { ty, name, init, .. } => {
            out.push_str(&format!("{}{} {} = {};\n", pad, ty, name, init));
        }
        CsStmt::Assignment { target, op, value, .. } => {
            out.push_str(&format!("{}{} {} {};\n", pad, target, op, value));
        }
        CsStmt::Expr(expr, _) => {
            out.push_str(&format!("{}{};\n", pad, expr));
        }
        CsStmt::If { cond, then_body, else_body, .. } => {
            out.push_str(&format!("{}if ({})\n", pad, cond));
            out.push_str(&format!("{}{{\n", pad));
            for s in then_body {
                emit_stmt(out, s, indent + 1);
            }
            out.push_str(&format!("{}}}\n", pad));
            if let Some(else_stmts) = else_body {
                // Check if it's an else-if
                if else_stmts.len() == 1 {
                    if let CsStmt::If { .. } = &else_stmts[0] {
                        out.push_str(&format!("{}else ", pad));
                        // Remove the pad from the nested if
                        let mut nested = String::new();
                        emit_stmt(&mut nested, &else_stmts[0], indent);
                        // Trim the leading whitespace
                        out.push_str(nested.trim_start());
                        return;
                    }
                }
                out.push_str(&format!("{}else\n", pad));
                out.push_str(&format!("{}{{\n", pad));
                for s in else_stmts {
                    emit_stmt(out, s, indent + 1);
                }
                out.push_str(&format!("{}}}\n", pad));
            }
        }
        CsStmt::Switch { subject, cases, .. } => {
            out.push_str(&format!("{}switch ({})\n", pad, subject));
            out.push_str(&format!("{}{{\n", pad));
            for case in cases {
                out.push_str(&format!("{}    {}\n", pad, case.pattern));
                for s in &case.body {
                    emit_stmt(out, s, indent + 2);
                }
            }
            out.push_str(&format!("{}}}\n", pad));
        }
        CsStmt::For { init, cond, incr, body, .. } => {
            out.push_str(&format!("{}for ({}; {}; {})\n", pad, init, cond, incr));
            out.push_str(&format!("{}{{\n", pad));
            for s in body {
                emit_stmt(out, s, indent + 1);
            }
            out.push_str(&format!("{}}}\n", pad));
        }
        CsStmt::ForEach { ty, name, iterable, body, .. } => {
            out.push_str(&format!("{}foreach ({} {} in {})\n", pad, ty, name, iterable));
            out.push_str(&format!("{}{{\n", pad));
            for s in body {
                emit_stmt(out, s, indent + 1);
            }
            out.push_str(&format!("{}}}\n", pad));
        }
        CsStmt::While { cond, body, .. } => {
            out.push_str(&format!("{}while ({})\n", pad, cond));
            out.push_str(&format!("{}{{\n", pad));
            for s in body {
                emit_stmt(out, s, indent + 1);
            }
            out.push_str(&format!("{}}}\n", pad));
        }
        CsStmt::Return(value, _) => {
            if let Some(v) = value {
                out.push_str(&format!("{}return {};\n", pad, v));
            } else {
                out.push_str(&format!("{}return;\n", pad));
            }
        }
        CsStmt::YieldReturn(value, _) => {
            out.push_str(&format!("{}yield return {};\n", pad, value));
        }
        CsStmt::Break(_) => {
            out.push_str(&format!("{}break;\n", pad));
        }
        CsStmt::Continue(_) => {
            out.push_str(&format!("{}continue;\n", pad));
        }
        CsStmt::Raw(code, _) => {
            for line in code.lines() {
                out.push_str(&format!("{}{}\n", pad, line));
            }
        }
        CsStmt::Block(stmts, _) => {
            for s in stmts {
                emit_stmt(out, s, indent);
            }
        }
        CsStmt::TryCatch { try_body, catches, finally_body, .. } => {
            out.push_str(&format!("{}try\n", pad));
            out.push_str(&format!("{}{{\n", pad));
            for s in try_body {
                emit_stmt(out, s, indent + 1);
            }
            out.push_str(&format!("{}}}\n", pad));
            for c in catches {
                out.push_str(&format!("{}catch ({} {})\n", pad, c.exception_type, c.name));
                out.push_str(&format!("{}{{\n", pad));
                for s in &c.body {
                    emit_stmt(out, s, indent + 1);
                }
                out.push_str(&format!("{}}}\n", pad));
            }
            if let Some(finally_stmts) = finally_body {
                out.push_str(&format!("{}finally\n", pad));
                out.push_str(&format!("{}{{\n", pad));
                for s in finally_stmts {
                    emit_stmt(out, s, indent + 1);
                }
                out.push_str(&format!("{}}}\n", pad));
            }
        }
        CsStmt::Throw(expr, _) => {
            out.push_str(&format!("{}throw {};\n", pad, expr));
        }
    }
}
