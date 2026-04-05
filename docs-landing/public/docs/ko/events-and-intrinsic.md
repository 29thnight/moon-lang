---
title: Events & Intrinsic
parent: 언어 가이드
grand_parent: 한국어 문서
nav_order: 11
---

# Events & Intrinsic

## 이벤트 리스너 sugar

PrSM은 Unity 이벤트 연결용 `listen` 문법을 제공합니다.

```prsm
listen startButton.onClick {
    SceneManager.loadScene("Game")
}
```

이 문법은 생성된 C#에서 `AddListener(...)` 호출로 lowering 됩니다.

## Intrinsic escape hatch

PrSM은 raw C# 삽입을 위한 `intrinsic` 형태도 포함합니다.

- 문장 블록 intrinsic
- 타입이 있는 intrinsic 표현식
- intrinsic 함수
- intrinsic 코루틴

이 영역은 일반 PrSM 의미 분석의 바깥에 있는 escape hatch 로 취급됩니다.
