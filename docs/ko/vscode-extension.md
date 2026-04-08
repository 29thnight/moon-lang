---
title: VS Code Extension
parent: 도구
grand_parent: 한국어 문서
nav_order: 3
---

# VS Code Extension

PrSM VS Code 확장(`prsm-lang`)은 `.prsm` 파일의 전체 편집 워크플로를 제공합니다: 구문 강조, 실시간 진단, 탐색, 코드 액션, 스크립트 구조 및 생성 C# 검사 도구.

## 설치

VS Code Marketplace에서 **PrSM** 검색, 또는 `.vsix` 파일에서 설치:

1. [GitHub Releases](https://github.com/29thnight/PrSM/releases)에서 `parkyoungung.prsm-lang-x.x.x.vsix` 다운로드
2. VS Code에서: **확장 > ··· > VSIX에서 설치**

MSI 인스톨러는 VS Code가 감지되면 확장을 자동으로 설치합니다.

## 언어 기능 (LSP 경유)

워크스페이스가 **신뢰됨** 상태일 때, 확장이 `prism lsp` 언어 서버에 연결하여 제공:

- **실시간 진단** — 입력 중 에러/경고 표시, PrSM 시맨틱 분석기 기반
- **정의로 이동** (F12 또는 Ctrl+Click) — 심볼 선언으로 점프, 크로스 파일 탐색 포함
- **모든 참조 찾기** (Shift+F12) — 프로젝트 전체에서 심볼 사용 위치 찾기
- **호버 정보** — 마우스 호버 시 타입 정보, 문서, 생성 C# 세부 사항
- **심볼 이름 변경** (F2) — 심볼과 프로젝트 전체 참조를 한번에 변경
- **문서 심볼** (Ctrl+Shift+O) — 현재 파일의 모든 선언 개요
- **워크스페이스 심볼** (Ctrl+T) — 전체 프로젝트 심볼 검색
- **코드 액션** — 명시적 제네릭 타입 인자 삽입, import 정리
- **자동완성** — Unity API (SQLite DB), 사용자 정의 심볼, 키워드

## 에디터 기능

### 구문 강조

55개 TextMate 스코프 — 키워드, 타입, 연산자, 문자열, 주석, 어노테이션, `listen`, `require`, `coroutine` 등 PrSM 전용 구문.

### 코드 스니펫

20+ 스니펫:

| 접두사 | 삽입 내용 |
|--------|----------|
| `comp` | Component 선언 스캐폴드 |
| `asset` | ScriptableObject asset 선언 |
| `func` | 파라미터가 있는 함수 |
| `cor` | wait가 있는 코루틴 |
| `listen` | Listen 블록 |
| `if` | If/else 블록 |
| `when` | When 패턴 매치 |
| `for` | 범위 기반 for 루프 |
| `ser` | Serialize 필드 |
| `req` | Require 필드 |

접두사를 입력하고 Tab을 누르면 확장됩니다.

### PrSM Explorer

워크스페이스의 모든 `.prsm` 파일을 보여주는 사이드바 트리 뷰. 클릭하여 열기, 상단 새로고침 버튼.

### Graph View (의존성 그래프)

컴포넌트 의존성 관계를 인터랙티브 그래프로 시각화합니다. `require`, `optional`, `child`, `parent` 필드를 통해 어떤 컴포넌트가 서로 참조하는지 보여줍니다.

열기: **Ctrl+Shift+V** 또는 명령 팔레트: `PrSM: Graph View`

### 스크립트 구조 시각화

현재 `.prsm` 파일의 내부 구조 — 선언, 필드, 함수, 라이프사이클 블록 — 을 WebView 패널에 표시합니다.

열기: 명령 팔레트: `PrSM: Visualize Script Structure`

### 생성 C# 보기

현재 `.prsm` 소스와 나란히 생성된 `.cs` 파일을 엽니다. `.prsmmap.json`을 사용해 생성 코드의 해당 위치로 점프합니다.

열기: **Ctrl+Shift+G** 또는 명령 팔레트: `PrSM: Show Generated C#`

### 원본 PrSM 소스 보기

생성 C# 보기의 역방향입니다. 생성된 `.cs` 파일을 보고 있을 때, 대응하는 위치의 원본 `.prsm` 소스로 돌아갑니다.

열기: 명령 팔레트: `PrSM: Show Original PrSM Source`

### 라이프사이클 블록 삽입

커서 위치에 라이프사이클 블록(`awake`, `start`, `update` 등)을 삽입하는 빠른 선택 메뉴.

열기: **Ctrl+Shift+L** 또는 우클릭 컨텍스트 메뉴: `PrSM: Insert Lifecycle Block`

### 스택트레이스에서 열기

에디터의 Unity/C# 스택트레이스를 파싱하여 소스맵 리매핑을 통해 원본 `.prsm` 소스 위치로 이동합니다. VS Code에 붙여넣은 Unity Console 출력에서 동작합니다.

열기: **Ctrl+Shift+T** 또는 우클릭 컨텍스트 메뉴: `PrSM: Open Source from Stack Trace`

### 컴파일 명령

| 명령 | 설명 |
|------|------|
| `PrSM: Compile Current File` | 활성 `.prsm` 파일 컴파일 |
| `PrSM: Compile Workspace` | 워크스페이스의 모든 `.prsm` 파일 컴파일 |
| `PrSM: Check Current File` | 출력 생성 없이 진단만 실행 |

## 키보드 단축키

| 단축키 | 명령 | 컨텍스트 |
|--------|------|---------|
| **Ctrl+Shift+G** | 생성 C# 보기 | `.prsm` 파일 활성 |
| **Ctrl+Shift+V** | Graph View | `.prsm` 파일 활성 |
| **Ctrl+Shift+L** | 라이프사이클 블록 삽입 | `.prsm` 파일 활성 |
| **Ctrl+Shift+T** | 스택트레이스에서 열기 | 모든 에디터 |
| F12 | 정의로 이동 | `.prsm` 파일 활성 |
| Shift+F12 | 모든 참조 찾기 | `.prsm` 파일 활성 |
| F2 | 심볼 이름 변경 | `.prsm` 파일 활성 |
| Ctrl+Shift+O | 문서 심볼 | `.prsm` 파일 활성 |

## 설정

| 설정 | 기본값 | 설명 |
|------|--------|------|
| `prsm.compilerPath` | `""` (자동 감지) | `prism` 바이너리 경로 |
| `prsm.checkOnSave` | `true` | 저장 시 진단 실행 |
| `prsm.showWarnings` | `true` | 경고 수준 진단 표시 |
| `prsm.unityApiDbPath` | `""` (번들됨) | Unity API SQLite 데이터베이스 경로 |

## Trusted Workspace

컴파일러 기반 기능(진단, 탐색, 이름 변경, 심볼, 컴파일 명령)은 **신뢰된 워크스페이스**에서만 활성화됩니다. 구문 강조, 스니펫, 기본 편집은 어디서나 동작합니다.

워크스페이스 신뢰: **파일 > 작업 영역 신뢰 관리**.

## 상태 표시줄

확장은 상태 표시줄에 현재 상태를 표시합니다:

| 상태 | 표시 |
|------|------|
| LSP 실행 중 | `$(check) PrSM (LSP)` |
| LSP 시작 중 | `$(sync~spin) PrSM (LSP)` |
| LSP 중지됨 | `$(warning) PrSM (LSP stopped)` |
| 레거시 모드 | `$(check) PrSM (legacy)` |
| 에러 발견 | `$(error) PrSM: N error(s)` |
| 경고 발견 | `$(warning) PrSM: N warning(s)` |
