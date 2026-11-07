---
id: sot:root/program-roadmap
title: Integrated roadmap and parallel work outcomes
status: active
created: 2026-11-06T23:55:19Z
updated: 2026-11-07T04:56:14Z
verify: manual
rel:
  depends_on: [sot:root/rust-migration, sot:root/client-collector, sot:root/adapter-authority, sot:root/delivery, sot:root/cross-os-runners, sot:root/chromium-lab-browser, sot:root/server-adapter-expansion, sot:root/browser-ui, sot:root/native-embedding]
---

# 전체 로드맵과 마일스톤

## 무료 원격 검증 적용 결정

사용자가 로컬 대신 비용 없는 원격 환경을 적용하도록 승인했다. 현재 gh 조회로
modootoday/humanymous의 PUBLIC visibility를 확인했다. 기본 실행 대상은 공개
저장소의 GitHub-hosted standard Ubuntu24.04이며 유료 larger runner, 자체 유료
서버 및 Cloudflare Containers 생성은 제외한다. Containers는 별도 유료 보조
검증 후보로만 남는다. 공개 표준 러너 compute 무료와 artifact/cache 저장 비용은
구분하고, 무료 경로는 기본적으로 작은 sanitized job log/summary를 사용한다.
민감한 private Specfold 원본이나 자격증명을 공개 artifact에 올리지 않는다.

CI owner는 기존 워크플로 재사용, 중복 실행 취소, bounded timeout/build parallelism,
무거운 Docker 검증 순차 실행 및 고정된 source SHA 결과 수집을 구현한다.
Root는27member workspace/lock과 각 owner source checkpoint를 통합한 후 rust
브랜치만 push하고 실제 hosted 실행을 관찰한다. dirty 로컬 파일을 원격 실행의
검증 소스로 주장하지 않는다. 기존 Docker topology/UID/egress/sandbox/witness
gate를 유지하고 component 성공과 full-product 완료를 구분한다. 최초 실제
무료 원격 실행 결과가 확인될 때까지 적용 상태는 implementation in progress다.

근거: https://docs.github.com/en/actions/concepts/billing-and-usage 및
https://docs.github.com/en/actions/reference/runners/github-hosted-runners .

### 로컬 개발과 hosted 실행의 역할 분리

기본 배치는 로컬 코딩·범위별 단위검사와 hosted 실제 런타임 검증으로 나눈다.
테스트 이름이나 #[test]/--lib만으로 순수 단위검사라고 분류하지 않는다. 실제
프로세스·포트·IPC·SCM·브라우저·권한 경계·Docker를 요구하면 component/runtime
검증이며 hosted 대상이다. 로컬은 fmt, 정적 검사, 범위 Clippy와 자원 상한이
있는 단위검사에 집중하고 Cargo 동시 작업은1개로 제한한다. 기존 메모리 보류를
무제한 로컬 빌드 허가로 바꾸지 않는다. 필요성이 입증된 로컬 런타임 디버깅은
별도 자원 배정과 기록을 요구하며 hosted 완료 증거를 대체하지 않는다.

| 구분 | 기본 실행 위치 | 완료 근거 |
|---|---|---|
| 코드·계약·단위·fmt·범위 Clippy | 로컬 | 명시적 package/feature와 실제 분모 |
| Node/Bun/nginx·실제 IPC·서비스·권한·restart | 표준 OS hosted | OS/arch/features별 actor 결과 |
| 제품 topology·격리·egress·witness·Firefox WASM | Ubuntu hosted Docker | 기존 Docker 필수 gate와 이미지·소스 해시 |
| CEF·SCM·설치기 OS 기능 | 지원 OS hosted | capability preflight와 실제 positive/negative |
| Pages 렌더·링크·browser smoke | hosted | 정확한 소스의 두 baseurl 및 실행 결과 |
| 사람·실제 AT/IME·기기/화면·물리 장애 | 적합한 별도 실환경 | 직접 증거; hosted label만으로 충족 불가 |

완전 직렬 CI는 독립 영역의 결과까지 막으므로 채택하지 않는다. 구현 checkpoint
4565026은 공통 policy 이후 두 lane으로 분리한다: A는 native→msrv→quality→
documentation→pages, B는 docker→browser→images→packages→node→advisories다.
각 matrix는 max-parallel1, 각 job의 Cargo는 jobs1이며 같은 lane의 이전 실패와
무관하게 policy 성공·미취소일 때 독립 결과를 계속 수집한다. ready는 항상
정확한 필수 job 결과를 집계하고 실패·skip·unknown을 성공으로 바꾸지 않는다.
이는 해당 workflow 내부 heavy 상한2다. 별도 수동 workflow까지 포괄하는 전역
상한으로 과장하지 않고, 동일 source의 중복 dispatch를 피한다. 공통 concurrency
group에 모든 job을 넣어 pending job이 대체·유실되는 방식은 사용하지 않는다.

변경 영향 기반 선택은 manifest/공통 의존의 전이 closure와 semantic 계약 변경을
검사할 신뢰 가능한 classifier가 준비된 뒤 도입한다. 현재는 필수 gate를 단순
paths 필터로 생략하지 않는다. Cargo/lock/toolchain/security/authority/runtime/
shared test-support 변경과 unknown 입력은 보수적으로 넓게 검증하며, release
candidate의 전체 필수 OS·product 분모는 계속 요구한다.

빌드 재사용은 같은 OS/arch/target/toolchain/lock/profile/features에서만 허용한다.
nginx native와 standalone pilot, Node host-pilot과 production package처럼 feature
graph가 다른 산출물을 합치지 않는다. CEF/전체 target/debug+release/Docker를 한
VM에 중복 축적하지 않고 runner 디스크 실측과 bounded context를 사용한다.
초기 무료 경로는 remote cache 쓰기와 일반 branch artifact 업로드를 끄고 작은
allowlisted summary·source/image/lock digest 및 job log를 사용한다. PUBLIC repo
artifact는 private vault가 아니다. 민감 raw는 업로드하지 않고 별도 승인된
비공개 회수 경로가 없으면 raw-not-recovered를 명시한다. LLM eval 추론/API비용은
무료 runner compute와 별개이며 이번 무료 실행 범위에 자동 포함하지 않는다.

버전1.2.0. 2026-11-07 KST 기준13개 프로젝트 채팅의 최신 turn, 기존9개
인계 기록, 현재 Git/spec/Actions를 대조한 전체 상태·순서·완료 조건의 정본이다.
실행 체크리스트는 plan:root/20261106235521, 근거는 evidence:root/20261106235519다.
이번 snapshot은 f32042a 원격 component 검증과 624c1c1 로컬 guard 구현을
구분한다. native CLI는 실제20번째 workspace member이며11unit/6process가
통과했다. SDK/framework runtime 지원은 아니다. 전체 목표는 미완료다.
완료율·완료일은 추정하지 않는다.

## 판정과 개발 규약

### 모든 채팅의 지속 정본화 의무

각 owner는 구현과 동시에 SoT의 계약/결정/schema/version/error/권한/lifecycle와
변경 delta, plan의 요구·의존·수용기준·미완료·후속 조치를 갱신한다. 채팅 간
API 합의는 메시지만으로 끝내지 않고 제안/수용/구현/검증 상태를 영속 기록한다.
실행 명령/source SHA 또는 dirty manifest/환경/tool·image·artifact digest/
실제 분모·실패·재시도는 private evidence에 남기고 실패 이력을 보존한다.
공개 evidence는 민감정보를 제거하여 Specfold attach로 등록하며 LF/Git bytes와
validated-working-source/canonical-Git-source 해시를 구분한다. checkpoint와
lane 양도에는 정확한 파일·계약·검증·잔여 한계·실행 명령·다음 owner를 남긴다.
타 owner 정본은 직접 수정하지 않고 change request로 전달한다. 본 의무는
root와13개 작업 채팅 모두에 적용되며 문서 갱신은 제품 구현/검증의 대체가 아니다.

| 상태 | 의미 |
|---|---|
| 구현·검증 | 지정 source와 하위 범위의 실제 실행 증거가 있음 |
| 부분 구현 | 코드가 있으나 제품 연결·필수 기능·자격화가 남음 |
| 진행 중 | 현재 작업/통합/실행 중; 미실행·미커밋 결과를 성공으로 합산하지 않음 |
| 연구·설계 | 조사·계약 정본화; runtime 지원·배포·표준 채택이 아님 |
| 미구현·미실행 | 필요한 코드 또는 실제 실행 증거가 없음 |
| 보류 | 명시된 선행 gate 충족 후에만 실행 |

채팅 completed는 담당 slice의 인계 상태다. notLoaded는 로딩 상태일 뿐 실패나
실행 상태가 아니다. 과거 green은 새 source를 보증하지 않는다. source SHA/dirty
manifest·이미지·환경·실행 분모를 귀속한다. mock,unsupported,ignored helper,
zero-test harness,tool error,CHALLENGE,실제 human ALLOW를 구분한다.

Rust1.99.1/edition2024,단일 책임·맥락별 모듈화·플랫 나열 금지·humanymous-*
네이밍·표준 Cargo 구조·중복 구현/코드 파편화 금지·단위/파일럿/e2e 의무를 유지한다.
신규 저작 구현/테스트는 Rust다. upstream/generated glue와 읽기전용 legacy oracle은
pin/provenance 및 최종 퇴역 조건으로 따로 관리한다. 새 외국어 구현 허가가 아니다.

scoring freeze,필수 증거 누락 fail-closed,파괴적 제어 dual-control,a11y의
motor/speed 비요구,owned lab 방어 전용 실험,T4 ceiling을 유지한다. DRM/CDM 부재,
UA 유사성,접근성 API 사용만으로 자동화 판정을 추가하지 않는다.

관리 기능은 local-only/인증된 로컬 peer 원칙이다. forwarded header만으로 원격
요청에 관리 권한을 주지 않는다. adapter의 보안 삽입·라우트 확장은 기존 서비스의
정상 header·중복Set-Cookie·body·stream/SSE·라우트 충돌과 장애 동작을 검증해야 한다.

## 검증된 기준점

| 영역 | 실제 근거 | 남는 제한 |
|---|---|---|
| workspace | root19packages,toolchain1.99.1;CEF native2개 포함 | 계획된 SDK/framework/CLI는 실제 등록 전까지 합산 금지 |
| 원본 이전 inventory | 314행:unit ported54/partial5/pending255,원본804함수 | 과거 단위 inventory;verified runtime parity0,최신 전체 구현률이 아님 |
| native6tuple | 46e941b/run42334814627 및 후속 f6d2893/run42335890743 성공 | implemented-crate 범위;full_runtime_qualified=false;5f90971 별도 실행 중 |
| Windows 실제 분모 | 46e941b 및 f6d full-job source에서51/51harness·214passed·쓰기거절 | raw2ignored custody helpers·zero harness 분리;최신 source에 소급 금지 |
| runtime/CLI Docker process | 실제 origin2/audit4,session/RIT·sealed custody·restart/replay/tamper,doctor/install/dryrun/rollback | bounded fixture;full Gate/browser/Pass/human/AT 아님 |
| collector | L1-L3 17signal,passive behavior,별도 env/advanced API;pure guard epoch | 최신 local native17/actual Firefox30;guard hooks·bootstrap/session 미완료 |
| 실제 원본 비교 | behavior16datasets 및 advanced.js 실제 browser report 비교 | test-only unchanged oracle;추가 OS/API error matrix·퇴역 남음 |
| Meari+CEF UI | 1a7de24 core18/native11·거부1case/4flags;별도96DPI gesture/CEF6readback/close 근거 | 마지막 DPI/max-edge/capture 수정은 unit/build-only;BrowserReady=false |
| Pages | 52c2c0d 및 후속 f6d full CI의 Ruby3.3.13 apex/subpath build·artifact 성공 | 최신 docs freshness/AT·production Pages deploy 별도 |
| 전체 delivery | f6d full CI 실패;Windows unit/actor/release build 후 필수 native runner 부재에서 실패 | full native/browser/Docker runner3개,docs freshness,registry/Go shipping 등 gate 남음 |
| 공급망 | 최근 기록된 Cargo audit587deps/known vulnerabilities0 | ttf-parser 유지보수 경고1;최종 Rust/native/image/배포 closure 재검증 필요 |

원시 로그·manifest는 private evidence,공개 요약은 해당 bundle의
custody-native-remote-results.json,windows-full-actor-results.json,
advanced-collector-results.json,guard-state-results.json,pages-remote-results.json이다.
cb5dce7 Linux custody 실패는 보존했다. 공용 exclusive guard 수정 뒤 다른 source의
성공을 과거 실패의 대체 기록으로 쓰지 않는다. collector256-task 실패/max2와
같은 이미지의512-task export1pass/max0도 별도 보존하며 sandbox를 해제하지 않았다.

5f90971 collector42338082218은 실제Firefox30/0failed/0ignored 성공이며 산출물을
다운로드했다. native42338082261은6환경모두성공이며report6개를다운로드했다.
해당Windows source는각51/51harness·228passed·raw2ignored helpers다.
CEF42338082212도component범위성공이다. full CI42338082293은별도관측하며 실행 중은성공이아니다.
이후full CI42338082293은실패로종료됐다. 실제native build/test/release 뒤
필수native runner 부재를거절했고browser/Docker runner·shipping/registry/docs
gate도미완료다. component 성공을full product 성공으로바꾸지않는다.

정본화 이후 구체적인 crates/native-cli manifest를 받아 working tree의20번째
workspace member로등록하고의존성을resolve했다. 새production dependency version은
없고기존serde/serde_json/junction을재사용한다. locked all-target check가통과했다.
source862883c와evidence:root/20261107042614를대조했으며root에서도unit11/process6·
Clippy가통과했다. Cargo/lock의귀속통합을완료한다. 현재workspace20이다.
이는위5f90971의19package기준점과구분한다. registry publish=false,SDK/framework
runtime지원은여전히미검증이다. 후속통합근거는roadmap-continuation-results.json이다.
현재source의manifest write는Windows 전용이고다른OS는dryrun 범위다. Cross-OS
완료로표시하지않는다. source/manifest/artifact 해시는최종인계기록과대조했다.
최신native6/collector/CEF요약은integration-5f-remote-results.json,
native CLI최종Cargo통합은native-cli-integration-results.json에귀속한다.

## 모든 병렬 채팅의 결과와 후속 책임

초기9개+후속 runtime/keys/browser3개+REA1개=13개 채팅을 확인했다. root는 이
13개와 별도로 공통 통합·collector를 수행한다. browser 채팅은 native CLI 인계 뒤
최신 사용자 승인에 따라 실제 newtab과 chrome:// 개별 지원 확인을 진행 중이다.
나머지는 마지막 반환 turn이 completed이며 notLoaded를 실행 상태로 해석하지 않는다.
세부 최신 snapshot은 roadmap-reconciliation-latest.json,이전 인계는 chat-results.json이다.

| 채팅/작업 | 귀속 결과 | 후속 작업 |
|---|---|---|
| Rust 연동 · Node/Bun 어댑터와 서비스 계약 | 6941032,Express/Hono14그룹·Node/Bun mock conformance | Rust-only binding·실제 authority·stream/provenance·npm/OIDC |
| Rust nginx · 플러그인 어댑터와 계약 검증 | b1e10ae,njs24·실제 nginx/mock authority28 | Rust plugin·실제 peer/ACL/replay/audit·정상 서비스 영향 검증 |
| Rust CLI · doctor와 안전한 install | fe26b4d,검증·dryrun/install/repeat/rollback;후속 공용 native security | 서명 artifact·서비스 활성화/readiness·OS별 실제 파일럿 |
| Rust 워게임 · attacker와 defender 격리 실험실 | 671e88a,공유Engine·seed 일치·12예약/8평가/4미지원 | live Gate/Pass/browser·human/AT·origin/audit 효과;HR-12 CHALLENGE 보존 |
| Rust Compose · 격리 제품랩과 Cloudflare 프로필 | 3b9c306,토폴로지26·실제authority+Node22 및 후속process pilot | full Gate/collector·egress·독립witness·provider 실행 |
| Rust authority · IPC 인증과 실제 어댑터 연결 | 6f5d83a,UDS/named pipe·peer/scope/replay·공유scoring/audit | typed provenance·stream·admin/RIT 전체 연결 |
| Windows 러너 · 표준 사용자 네이티브 테스트 | c6a2457/99cf472 기반;후속6tuple·full-job actor 검증 | full native product/service/peer 파일럿·최종 source |
| Rust CI/CD · Cross-OS와 OIDC 릴리스 | 44e79a4,wiring·docs review 도구;후속Rust helper/CI 수리 | 남은 legacy tooling Rust 이전·runner3·registry trust·실제 release gates |
| Rust docs · Pages와 공개 Draft v0 | 2d98934,77문서·과거88browser조합/keyboard8;후속remote Pages build | 최신source review·AT·표준 wire/vectors·실제 Pages serving |
| Rust 런타임 · Core/Gate 세션과 collector | 9ce3190,실제session/ingress/RIT/audit-before-origin;rootsealed startup/Docker 통합 | raw TLS/H2·qualified collector·proxy/admin/Pass |
| Rust 키 저장 · sealed custody와 vault 영속성 | 5a77605,Windows49/Linux47;root공용guard·WAL 수리/6tuple | rotation운영·독립durable witness·dual-control issuer·backup/erasure |
| Meari + CEF · Rust 검증 브라우저 UI | 1a7de24 실제UI,7e2d848 SDK/framework 설계;현재CLI 명명·구현 재개 | B2잔여자격화,B3receipt,B6SDK/ABI/framework/CLI실행 |
| 내장 브라우저 분석 작업 | 512a73f bounded REA 정적 embedding 분석·관련 답변 | 참고 관찰만;Owl 전체복제/클린룸 구현 제외;DRM 일반화 금지 |

SDK 기반4crate(native-core/windows/ffi/plugin)와 후속 Electron main-process
Node-API·Tauri/winit 후보·generic C ABI consumer·native CLI는 구분한다. browser
실험용 process-owning DLL을 SDK로 재수출하지 않는다. 명명 보정은
humanymous-native-cli / humanymous-native init --framework electron이다.
init/doctor/verify의 실제 구현·membership은 코드와 통합 증거가 생길 때 갱신한다.
Rust-only 조건상 생성기가 JS/TS 등 외국어 템플릿을 만드는 우회는 허용하지 않는다.
기존 외부 호스트의 연결 책임과 검증된 packaged OS/arch/runtime 경계를 명시한다.

## 검증 이력 타임라인

아래 시간은2026-11-07 KST의 실제 귀속 commit 시점이며 CI 완료 시각과 다르다.

```mermaid
timeline
  title Humanymous 구현 이력과 현재 기준
  07시-08시 병렬 기반 : Node/Bun와 nginx 계약 : CLI와 워게임 : authority와 Compose : CI와 Pages
  09시 후속 통합 : REA bounded 분석 : sealed custody : Core Gate session
  12시 검증 기반 수리 : custody WAL 잠금 : collector target : Pages Ruby : Windows actor
  12시51분 수집 확장 : Rust environment API
  13시12분 고급 관측 : advanced API와 실제 원본 비교
  13시19분 브라우저 UI : borderless SVG와 resize
  13시21분 guard 상태 : coverage와 bounded epoch
  13시23분 native 설계 : SDK framework와 init tooling
  13시47분 이후 : f32042a native 6환경과 collector CEF 성공 : full CI 실행 중
  13시56분 이후 : 624c1c1 guard baseline 실제 관측 : native19 Firefox34 로컬 통과
  14시 이후 현재 : newtab과 개별 chrome URL 구현 진행 : root 전체 로드맵 재정합화
```

## 목표 완결 마일스톤

| ID | 범위 | 상태 | 완료 조건/남은 작업 |
|---|---|---|---|
| M0 | branch·Specfold·개발 규약 | 구현 | .spec/.spec.local·귀속·증거 유지,최종 agents정리는M11 |
| M1 | 순수엔진·모델·Pass | 부분 | 원본 회귀 전수·ML lifecycle·full Pass·실제runtime parity |
| M2 | network/raw capture | 부분 | TLS ClientHello·ordered headers·HPACK/H2설정/priority·connection/session binding |
| M3 | audit/keys/state | 부분 | 독립durable witness·rotation운영·dual-control·backup/erasure/sinks |
| M4 | Core/Gate·authority | bounded runtime | collector/capture/RIT·proxy/settings/abuse/admin 전체 실제 경로 |
| M5 | Rust collector | baseline까지 부분 | native19/Firefox34 로컬; guard hooks/CDP·fingerprint-ID/ICE·loader/session/RIT/recovery·Pass·API matrix |
| M6 | 서버 adapters | 부분/연구 | Node/Bun Express/Hono·nginx Rust-only,peer/route/header/body/stream/provenance와실제authority |
| M7 | 운영 CLI·설치 | provisioning 구현 | signed package→서비스 활성/readiness→업데이트/rollback·native peer 부정 파일럿 |
| M8 | 제품 Docker 워게임 | 제한된 파일럿 | live Gate/browser/Pass·egress·origin/audit·공격/정상/human/AT 분모 |
| M9 | 자동화/모바일/provider/표준 | 연구·draft | owned 실행·wire/vectors·상호운용/security/privacy/a11y 검토·국제 제출 준비 |
| M10 | Cross-OS·OIDC·Pages | scoped검증/wiring | full runner3개·최종source6tuple·registry/package trust·docs freshness·Pages artifact;배포확인은M12 |
| M11 | Rust-only·공급망·agents/skills | 부분 | foreign-source 전수 disposition·CVE/SBOM/image scan·.agents/rules/skills/agents와사용자skills/eval |
| M12 | 최종 main만 유지 | 보류 | 최신main 통합candidate→exact SHA/tree전체gate→main갱신→Pages실제배포확인→rust branch제거 |
| B0/B1 | 브라우저 설계/engine/profile | 설계·부분 | pin/provenance·권한/network/lifecycle 자격화 |
| B2 | Meari+CEF 실제UI | Windows부분 | full sandbox/input/IME/AX·DPI/crash·OS tuples·Chrome rendering 기능matrix |
| B3 | 진단/receipt | 미구현 | qualified target/input·collector/session/capture/audit typed join |
| B4 | 대조 실험 | 미실행 | ordinary Chrome·human/AT·observer on/off·오탐 및 실제효과 |
| B5 | 도구 배포 | 미구현 | native CVE/SBOM·서명/update/rollback·사용자문서 |
| B6 | native SDK/framework/CLI | 설계/manifest init 구현 | CLI11unit/6process; 기반4crate·versionedABI/consumer·owned lifecycle/unload·frameworkmatrix·scaffold/doctor/verify |

## 앞으로의 실행 타임라인

W는 완료일이 아닌 의존 순서다. 설계 병렬 가능과 작업 배정/실행 완료는 다르다.

```mermaid
flowchart LR
  W0[현재 W0<br/>이력 정합화와 최신 SHA CI] --> W1[W1 제품 필수 producer 완성]
  W1 --> W2[W2 실제 서비스 연결]
  W2 --> W3[W3 제품 전체 검증]
  W3 --> W4[W4 배포 후보 검증]
  W4 --> W5[W5 main 전환과 공개 확인]
  G[guard·fingerprint·ICE] --> W1
  C[raw TLS H2 capture] --> W1
  A[witness·rotation·dual-control] --> W1
  B[병렬 브라우저·native SDK·CLI] --> W3
  D[지속 문서·표준 draft·adapter 계약] --> W4
```

| 파동 | 다음 결과 | 병렬 경계 | 통과 증거 |
|---|---|---|---|
| W0 | 13채팅 재확인,20crate/CLI 통합,guard baseline, f32042a component 성공/fullCI 관측 | root공통Cargo/CI/Git;browser자기lane | source·로그·실행분모,실패보존 |
| W1 / F01-F02/F10-F11 | guard baseline/observer/forwarding,rawcapture,키 운영;UI/SDK/init 병렬 | collector/network/audit/browser 별도writer,scoring seam직렬 | 실제 producer·누락거절·cleanup/재시작·권한/ABI·API/OS matrix |
| W2 / F03-F05 | qualified collector/RIT,서버 Rust binding,서비스 활성화 | 계약고정 뒤 adapter/CLI 분리 | 실제authority/origin/audit 효과·stream/header정상성·readiness/rollback |
| W3 / F06 | full native/browser/Docker Rust runner와live워게임 | lab/CI와도구SDK 경계 분리 | six tuple·제품Docker e2e·human/AT·observer대조·SDK consumer |
| W4 / F07-F09 | 최종docs·Pages artifact·OIDC/registry·공급망·skills | docs/.agents/packaging분리,후보source고정 | 같은sourcefreshness·archive검토·trust·CVE/SBOM·실제modoo skills eval |
| W5 / M12 | oracle/Go/폐기물 퇴역,최신main통합candidate검증,main갱신·Pages배포확인·branch제거 | Git전역직렬 | exactcandidate SHA/tree전체gate,실제Pages serving,미완료시branch제거보류 |

M12의 순환 의존성을 만들지 않는다. Pages는 먼저 후보 artifact를 검증하고,
main-triggered 실제 deployment는 main 갱신 후 확인한다. 변경될main을 먼저
candidate에 반영하고 최종gate 뒤 새미검증merge commit을 만들지 않는 전환을
설계한다. 배포 실패 시 main-only 완료를 주장하거나 복구 branch를 먼저 지우지
않는다. registry 실제발행·tag는 준비된artifact와해당release운영규약에귀속한다.

## 연구 backlog의 실행 전환

| 조사 | 정본 방향 | 필요한 후속 증거 |
|---|---|---|
| 서버 adapter/언어/OS | Node/nginx 서비스 통합 우선;SDK framework와별도 bounded context | Rust binding과지원주장별실행matrix |
| Computer Use/Grok/Meta/Pickle/crawler/DC IP/API/mobile | actor/egress/authorization/provenance 구분,robots위반추정금지 | bounded eventtracking·owned재현·API/nonbrowser·AT controls |
| Android Monkey 등 | 모바일 자동화 위협/관측 분리 | owned device OS/driver/실제입력·정상 대조 |
| DRM/CEF/Electron/desktop-use | capability·license/CDM/build 차이,단독bot규칙금지 | 허용된 capability matrix·observer개입·실제원본브라우저 대조 |
| Actions/전용runner | native6tuple가능 | GUI/service/browser/remotepeer/full delivery 실행 |
| Compose/Cloudflare/AWS/Lightsail | 격리 profile/1회성lab | 비용·권한·egress·teardown 및 실제provider실행;현재not_run |
| 국제 웹 표준 draft v0 | 체계적 작성·검토·등록준비,docs/Pages공개영역 | wire/vectors·상호운용·security/privacy/a11y·기관절차 검토 |
| crate/native/framework 설계 | humanymous-*·표준Cargo·책임분리·공유엔진재사용 | 실제membership/ABI/OS/frameworkconsumer와의호환성 |

표준 문서는 docs 하위 Pages 서빙 대상이다. 내부spec/private원문은 공개 docs로
그대로 복제하지 않는다. modootoday Jekyll remote_theme·기존디자인토큰 이식을
유지하고,작성·빌드·실제배포·표준기관제출/채택을각각구분한다. Cloudflare는
Wrangler config 없는 경우cf CLI를사용한다. 이번 정리는 provider생성을 수행하지 않는다.

## 현재 종료를 막는 gate와 다음 착수

현재client Session은strict provisionedbootstrap/oneflight/sharedsigner/zeroizing
keyepoch/clockexpiry/ACK확인후counterprogress를구현했다. Runtimeowner가ACK계약을
수용했지만serverwire구현은별도다. collector26/integrity20+parity3,actualFirefox58,
host/WASMClippy와독립.NET ACK벡터가통과했다. 이는actualFetch/completebody/abort/
serverbootstrap/session/freshcollectorjoin을대체하지않으며전송계획은
plan:root/20261107055000이다. root/bodycodec1MiB와초기server64KiBbudget도분리한다.

현재explicit probe_cdp_json은legacy console-preview 두관측을actualRust로
수행하며once-per-collectorcache/status/error/latecallback/stop을검증했다.
Native19/actualDockerFirefox51/strictWASMClippy는component범위이며CDP종류나
자동화/humanidentity를보증하지않는다. 기존frozenflag evaluator만재사용한다.
공통workspace는작업트리27members다.25member시점의delivery/M11tooling은
rootlock검증(관리7unit4process+delivery16,Clippy)을통과했고이후runtime-pilot과
nginx-adapter를추가해Windowsdefault cargo check가통과했다. Linuxnative nginx
실행자격화는아니다. 별도M11lock은private원본보존후제거했으며공통manifest
귀속/나머지worker최종source통합은아직진행중이다.

Guard는실제prototype/native baseline과ownedMutationObserver를구현했다.
directSCRIPT/falsyhm만집계하고대기recorddrain/stop/drop/과대입력/재진입/
실패disconnect를검증한다. enable_guard_hooks는명시opt-in이며default생성자는
전역eval/Function을대체하지않는다. directeval lexical변경을API/정본에명시하고
return/throw/this/newTarget/중첩호출/부분rollback/외부교체보존/복원실패/
동일realm중복거절도현재51개browsercase에포함된다. defaultguard.reported=false와
opt-in measuredcoverage를구분하며인간/코드실행attestation으로사용하지않는다.
Windows컴파일WASM을기존고정Linuximage에readonlymount한component증거로서
새sourceimagebuild나full제품e2e는아니다. 과거34/40/47분모는각sourceevidence에
보존하며현재결과와혼합하지않는다.

### Browser request codec 구현 경계

공유 integrity::request::browser의 Binding/canonical/BindingError/PROTOCOL은
실제 구현됐다.14field byte-length framing과별도domain,rawbodySHA256,
canonicalsession/JSsafe숫자/경로/헤더/buffer bound를검증한다.17unit+3legacy
parity/strictClippy/wasm32check가통과했고고정벡터SHA/HMAC는독립.NET검산과
일치한다. 이는Fetch실행·bootstrap·freshsession·replay·qualifiedreport나
제품e2e가아니다. runtime과collector가같은codec를직접소비해야하며canonical은
applicationcredentials를포함하므로로그/진단/공개증거에노출하지않는다.
초기cookie-freeprofile과추후route-scopedcookieprepare지원은구분한다.

### 공통 adapter parser의 provisioned identity 계약

공유 integrity parser는 고정 protocol0.1의 검증을 복제하지 않고
parse_for(bytes,now,ExpectedAdapter)를 제공한다. expected id/version은 인증된
설치 설정에서만 전달하고 요청 envelope에서 선택하지 않는다. 기존 parse는
기존 Node identity/version에 고정하여 호환성을 유지한다. 다른 identity를
명시적으로 설정해도 raw TLS/H2/capture/stream 지원은 생기지 않으며 기존
필수 필드·중복키·digest·deadline·provenance 거절을 모두 유지한다.
ValidatedRequest의 identity/header mode/connection ID getter는 관측값이며
peer 인증·scope/replay·audit 권한을 대신하지 않는다. 잘못된 설정 identity,
version mismatch 및 non-null capture/stream을 부정 검증한다.

후속 알려진 profile은 parse_with_profile과 AdapterProfile::{Node,Nginx}로
명시한다. Nginx는 nginx-raw/nginx-normalized만 허용하며 connectionId는 항상
null이다. normalized는 peer도null, rawHeaderOrder=false다. raw는 파싱된 HTTP
헤더 순서 관측일 뿐 TLS/H2/capture나 원본 client 인증을 증명하지 않는다.
기존 parse/parse_for의 관측 vocabulary는 유지한다. 새 profile 검증의 local
integrity13unit+3legacy parity와 strictClippy가 통과했으나 native nginx plugin과
authority의 credential/profile 결합은 담당 채팅의 후속 구현이다.

1. 전체 rawcapture/qualified collector/Pass/proxy/admin 경로가 없다.
2. audit d001ee6의witness/control·CLI83397fd의signedservice 경계를실제runtime/
   OS운영/독립authority에연결하고자격화해야한다. API구현을미구현으로재분류하지않는다.
3. native/browser/Docker Rust runner구현의제품전체연결·exactsource OS실행과
   실제human/AT e2e가남았다. 기존component/nativefixture를전체성공으로바꾸지않는다.
4. 브라우저·native SDK/framework의sandbox/ABI/OS/AT 자격화가 미완료다.
5. 최신docs review/production Pages·registry·최종scan·Rust-only/agents/skills/eval이 남았다.
6. 모든요건 전에 Go/oracle을지우거나main병합/branch삭제/publish를완료처리하지않는다.

다음root구현은fingerprint/ICE와clientfreshsession/transport,공통27member통합이다.
공통Cargo/lock/toolchain과최종통합은root,CI는지정delivery채팅,나머지는각단일writer다.
사용자의명시요청으로기존13채팅모두후속을재개했고새표준채팅은만들지않았다.
기존Pages/draft-v0채팅이전체공개문서와표준지속goal을맡는다. 각채팅의계약/
이력/실행근거지속정본화의무는7827477에기록됐으며대기R01-R17는rootprivate
runtime-waiting-contracts.json에owner-reported상태로보존했다.

공개 Specfold와private 전체tree상태는분리한다. browserPNG의자동secret-scan미지원
U2는manualsynthetic시각검토범위와함께보존하며면제하지않는다. 과거profile삭제의
자동승인거절·미삭제기록도이력으로유지하며새삭제완료로주장하지않는다.
