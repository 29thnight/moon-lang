# Moon 컴파일러 아키텍처

## 컴파일 파이프라인

```
.mn 소스
  → Lexer (토큰화)
    → Parser (AST 생성)
      → Semantic Analysis (이름 해결, 타입 체크, null safety)
        → Lowering (AST → C# IR)
          → CodeGen (C# IR → C# 소스 텍스트)
            → .cs 파일 출력
              → Unity C# 컴파일러 → IL → Mono/IL2CPP
```

## 구현 언어

**Rust** — 네이티브 성능, 단일 바이너리, 강력한 enum/패턴매칭

## 주요 모듈

### Lexer (`lexer/`)
- `logos` 크레이트 기반 토큰화
- 키워드, 연산자, 리터럴, 문자열 보간, 시간 접미사 (`1.0s`)
- 줄/열 위치 추적

### Parser (`parser/`)
- 재귀 하강 파서
- 괄호 없는 if/when/for/while 처리
- intrinsic 블록: 중괄호 매칭만 (내용 파싱 안 함)
- 에러 복구 및 다중 에러 보고

### AST (`ast/`)
- Rust enum 기반 노드 트리
- 모든 노드에 SourceSpan 포함
- 불변 트리 (수정 시 새 트리 생성)

### Semantic (`semantic/`)
- 4단계: 선언 수집 → 임포트 해결 → 멤버 해결 → 본문 분석
- 스코프 기반 심볼 테이블
- Null safety: 스마트 캐스트, `?.` 전파
- `when` 완전성 검사

### Lowering (`lowering/`)
- AST → C# IR 변환
- Awake 메서드 조립 (require/optional/child/parent)
- 코루틴 → IEnumerator 변환
- sugar 매핑 (vec3, input, listen 등)

### CodeGen (`codegen/`)
- C# IR → 포맷된 소스 텍스트
- 들여쓰기, 헤더 주석, #line 디렉티브

### Diagnostics (`diagnostics/`)
- `ariadne` 크레이트 기반 에러 리포팅
- 에러 코드 체계 (E001~, W001~)
- JSON 출력 모드 (Unity 통합용)

## Unity 통합

1. `moonc.exe compile <file.mn>` → `.cs` 출력
2. Unity ScriptedImporter가 `moonc.exe` 호출
3. 생성된 `.cs`는 `Assets/Generated/Moon/`에 배치
4. Unity가 표준 C# 컴파일 수행
