# Common Investigation Hotspots

다음 항목은 이 저장소에서 반복적으로 경계면 버그를 만들었던 지점이다.

## VS Code binary resolution

- 개발 중에는 `vscode-prsm`이 확장 번들 바이너리보다 워크스페이스 빌드 `prism`을 우선해야 한다.
- 여기서 어긋나면 hover, definition, references, rename, symbol 결과가 오래된 컴파일러 기준으로 보일 수 있다.

## Project-root assumptions

- `vscode-prsm`의 generated output lookup은 현재 문서의 프로젝트 루트를 기준으로 해야 한다.
- `workspaceFolders[0]`만 보면 BlazeTest 같은 외부 프로젝트에서 type hover나 navigation이 어긋날 수 있다.

## Source-map chain

- compiler가 `.prsmmap.json`을 생성한다.
- `vscode-prsm`과 `unity-package`가 그 sidecar를 소비한다.
- line span, nested statement segment, remapped path 중 하나라도 어긋나면 generated C# navigation이나 runtime stack trace remap이 깨진다.

## Legacy compatibility

- `.mnproject`와 혼합 `.mn`/`.prsm` 경로 호환이 남아 있다.
- repo에서는 통과하지만 외부 프로젝트에서는 옛 설정이나 stale manifest 때문에 깨질 수 있다.

## BlazeTest smoke environment

- `run-blazetest-smoke.ps1`는 외부 `C:\Users\idene\BlazeTest` 프로젝트를 기본으로 본다.
- Unity 에디터가 이미 열려 있으면 스모크가 환경 이유로 막힐 수 있다.

## Packaged artifact drift

- `vscode-prsm/bin/prism.exe`는 추적되는 패키징 산출물이다.
- 로컬 빌드와 번들 바이너리가 불일치하면 설치 스모크나 패키징 검증에서만 드러나는 문제가 생길 수 있다.