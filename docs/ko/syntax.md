---
title: Syntax
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 1
---

# Syntax

PrSM의 표면 문법은 의도적으로 작게 유지됩니다.

- 파일당 하나의 최상위 선언
- 보통 `using` 임포트로 시작
- 세미콜론 없이 줄바꿈으로 문장 종료
- 괄호 없는 중괄호 기반 제어문
- 생성된 C#이 원본 구조와 가깝게 유지됨

기본 파일 형태:

```prsm
using UnityEngine

component PlayerController : MonoBehaviour {
    update {
    }
}
```
