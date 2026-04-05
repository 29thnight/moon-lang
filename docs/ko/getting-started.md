---
title: 시작하기
parent: 시작하기 (KO)
nav_order: 2
---

# 시작하기

## 컴파일러 실행

```powershell
cargo test
cargo run -p refraction --bin prism -- compile samples\PlayerController.prsm --output build-output
cargo run -p refraction --bin prism -- check samples\PlayerController.prsm
```

## 프로젝트 초기화

```powershell
cargo run -p refraction --bin prism -- init
```

이 명령은 프로젝트 메타데이터, 소스 glob, 출력 경로를 담는 `.prsmproject` 파일을 생성합니다.

## 분석 명령 확인

```powershell
cargo run -p refraction --bin prism -- hir . --json
cargo run -p refraction --bin prism -- definition . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- references . --json --file samples\PlayerController.prsm --line 10 --col 5
cargo run -p refraction --bin prism -- index . --json
```

## 에디터 도구 검증

```powershell
cd vscode-prsm
npm install
npm test
```
