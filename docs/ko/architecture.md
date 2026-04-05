---
title: Architecture
parent: 내부 구조
grand_parent: 한국어 문서
nav_order: 1
---

# Architecture

현재 컴파일 파이프라인은 다음과 같습니다.

```text
.prsm source
  -> Lexer
  -> Parser
  -> Semantic Analysis
  -> Lowering to C# IR
  -> C# emission
  -> .cs + .prsmmap.json output
```

현재 주요 모듈 역할:

- `lexer`: 토큰화, 문자열 보간, duration literal, source position
- `parser`: 재귀 하강 파싱과 에러 복구
- `semantic`: 심볼 해결, 타입 검사, null-safety, 선언 검증
- `lowering`: AST 에서 C# IR 로 변환
- `codegen`: 포맷된 C# 생성
- `driver` 및 project graph/index helper: CLI orchestration
