# PrSM 구현 로드맵

## 마일스톤 0: 기반 구축 ✅
- [x] v0 언어 스펙 작성
- [x] 전체 아키텍처 플랜 작성
- [x] Cargo 워크스페이스 스캐폴드
- [x] 문서 파일 생성
- [x] 샘플 .prsm 파일

## 마일스톤 1: 렉서 ✅
- [x] TokenKind 정의 (키워드, 연산자, 리터럴)
- [x] Lexer 구현 (logos 기반)
- [x] 문자열 보간 토큰화
- [x] 시간 접미사 (1.0s) 처리
- [x] 60개 테스트 케이스

## 마일스톤 2: 파서 ✅
- [x] AST 노드 정의
- [x] 재귀 하강 파서
- [x] 괄호 없는 제어문 파싱
- [x] wait DSL 파싱
- [x] listen 이벤트 파싱
- [x] intrinsic 블록 파싱
- [x] AST 프리티 프린터
- [x] 에러 복구

## 마일스톤 3: 의미 분석 ✅
- [x] 심볼 테이블
- [x] 이름 해결
- [x] 타입 체크
- [x] Null safety 분석
- [x] 선언 검증
- [x] when 완전성 검사
- [x] 확정 할당 분석

## 마일스톤 4: 로우어링 ✅
- [x] C# IR 노드 정의
- [x] AST → C# IR 변환
- [x] Awake 조립
- [x] 코루틴 lowering
- [x] sugar 매핑 (vec3, input, listen)
- [x] intrinsic verbatim 출력

## 마일스톤 5: 코드 생성 ✅
- [x] C# IR → 소스 텍스트
- [x] 포맷팅/들여쓰기
- [x] #line 디렉티브
- [x] 골든 파일 테스트
- [x] 생성된 C# 컴파일 검증

## 마일스톤 6: CLI ✅
- [x] `prism compile <file>`
- [x] `prism check <file>`
- [x] 배치 컴파일 (`prism build`)
- [x] 진단 메시지 포맷 (텍스트/JSON)
- [x] `prism init`, `prism where`, `prism version`
- [x] Watch 모드 (`prism build --watch`)

## 마일스톤 7: Unity 에디터 패키지 ✅
- [x] ScriptedImporter (`.prsm` 파일 자동 임포트)
- [x] PrismCompilerBridge (`prism where`로 바이너리 탐색)
- [x] AssetPostprocessor (파일 변경/삭제/이름변경 감지)
- [x] 커스텀 인스펙터 (PrismComponentEditor, PrismScriptInspector)
- [x] 템플릿 (component, asset, class)
- [x] 컨텍스트 메뉴 (Compile/Check/Build)
- [x] 생성 코드 패키지 (`com.prsm.generated`) 구조
- [x] Unity Console 진단 매핑 (클릭→소스 이동)

## 마일스톤 8: VSCode 확장 (v1.1) ✅
- [x] TextMate 구문 강조 (59개 스코프)
- [x] 실시간 진단 (`prism check --json` 연동)
- [x] 코드 스니펫 20개
- [x] 사이드바 탐색기
- [x] 그래프 뷰 (컴포넌트 관계 시각화)
- [x] 라이프사이클 삽입 (Ctrl+Shift+L)
- [x] Unity API 자동완성 (SQLite DB)
- [x] C# DevKit 연동
- [x] trusted workspace `prism lsp` 경로 (completion, definition, hover, references, rename, document/workspace symbols)
- [x] VSIX 패키징 완료

## 마일스톤 9: 통합 테스트 (진행 중)
- [x] BlazeTest Unity 프로젝트 구성 (Unity 6.0.4)
- [x] `.prsmproject` 기반 프로젝트 빌드
- [x] TestScript.prsm — serialize, when, 함수, 로깅
- [x] DirTestScript.prsm — 컴포넌트 상속
- [x] 생성 C# → Unity 컴파일 성공
- [x] `unity-package/Tests/Editor` 패키지 내부 EditMode 테스트 기반 추가
- [ ] 네거티브 테스트 케이스 확충
- [x] 추가 문법 커버리지 (listen, intrinsic 등)

## 다음 단계
- [x] 패키징 / 설치 검증 자동화 (VSIX 에 최신 `prism.exe` 와 번들이 들어갔는지 확인)
- [ ] 네거티브 테스트 케이스 확충
- [ ] 디버깅 / source-map 워크플로 확장
- [x] 실제 Unity 프로젝트 기준 end-to-end 검증 확대
