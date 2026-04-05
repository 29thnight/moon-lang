---
title: Functions
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 8
---

# Functions

현재 PrSM 함수는 다음을 지원합니다.

- 블록 본문 함수
- 표현식 본문 함수
- 명시적 반환 타입
- `private`, `protected`, `public`
- `override`

예시:

```prsm
func jump() {
    print("jump")
}

func isDead(): Bool = hp <= 0
```

클로저는 아직 별도의 큰 언어 장으로 정리할 정도의 사용자 기능으로 노출되어 있지 않습니다. 다만 `listen` lowering 과정에서는 이벤트용 람다가 생성됩니다.
