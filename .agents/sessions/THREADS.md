# Project chat ownership registry

Snapshot: 2026-11-07 KST. Chat active status is distinct from persistent goal status. Each owner must inspect get_goal; unfinished goals are preserved. New goals below are dispatched, not presumed accepted or completed.

| Chat | ID | Responsibility |
|---|---|---|
| Humanymous · 통합 · Rust 전환과 검증 | 01a1b25e-8e4c-7a70-b7b7-f51a66c58514 | Existing owned context; preserve current claims and plans |
| Humanymous · 권한 · IPC와 어댑터 인증 | 01a1b30d-454e-7a50-9ab1-48358dbd0ff1 | Existing owned context; preserve current claims and plans |
| Humanymous · 개발도구 · 규칙과 Skills 검증 | 01a1b34e-f9e9-77d3-b883-0cc4d76763a4 | Existing owned context; preserve current claims and plans |
| Humanymous · 문서 · Pages와 Draft v0 | 01a1b2da-2656-7a23-a60b-08a681b23117 | Existing owned context; preserve current claims and plans |
| Humanymous · 런타임 · Gate와 서비스 수명주기 | 01a1b35f-cf83-74f2-aad1-34b3aee60cda | Existing owned context; preserve current claims and plans |
| Humanymous · 워게임 · 공격·방어 검증 | 01a1b2f5-ec55-7c91-863d-f539c70417be | Existing owned context; preserve current claims and plans |
| Humanymous · 어댑터 · nginx | 01a1b2f5-ea98-7023-b7af-ad527b820084 | Existing owned context; preserve current claims and plans |
| Humanymous · OS 보안 · 자격증명과 Windows 검증 | 01a1b30d-439d-7030-8e38-789bc0b70f6a | Shared native-security credential providers and ordinary-actor Windows verification; no duplicate OS secret reader |
| Humanymous · CLI · 진단과 설치 | 01a1b2f5-e927-7a03-9d47-9d1084d14c53 | Existing owned context; preserve current claims and plans |
| Humanymous · 브라우저 · Meari와 CEF | 01a1b35f-d3e9-7042-91a4-10c910a15515 | Existing owned context; preserve current claims and plans |
| Humanymous · 감사 · 키 보관과 영속성 | 01a1b35f-d1a1-7553-a0ed-65d09b9042a7 | Existing owned context; preserve current claims and plans |
| Humanymous · 어댑터 · Node·Python·Ruby | 01a1b2da-d1e9-7a22-9052-9a90a8cba54f | Rust host/framework bindings for Node/Bun, Python and Ruby; shared authority/ABI and actual hosted framework qualification |
| Humanymous · 제품랩 · Compose와 브라우저 격리 | 01a1b30d-4736-7281-a54f-5cb3a5fa4fe4 | Owned Compose topology, isolated browser/product runners and independent effects; provider deployment is separate |
| Humanymous · CI/CD · Cross-OS와 OIDC | 01a1b2da-7d8a-73f2-9bb2-78bde5ae33f9 | Existing owned context; preserve current claims and plans |
| Humanymous · Apache · httpd 모듈 | 01a1b4e3-b0f1-7320-81a8-13e89d1d0dff | adapters/apache/**,crates/apache-adapter/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · 네이티브 ABI · C와 C++ 연결 | 01a1b4e3-b2b7-71d3-bf3b-66a851cb7cf4 | crates/native-abi/**,adapters/native/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · Go · HTTP Gate 호환성 | 01a1b4e3-b4a2-7690-9057-24b06d04b652 | crates/go-adapter/**,adapters/go/**; Rust-owned Go HTTP/Gate compatibility boundary and hosted consumer evidence |
| Humanymous · .NET · ASP.NET Core 연결 | 01a1b4e3-b687-7822-88c3-20d053f7d8fd | crates/dotnet-adapter/**,adapters/dotnet/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · 네트워크 · TLS와 HTTP2 캡처 | 01a1b4e3-b852-7d21-90d9-4f00bebb5420 | crates/capture/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · 워터마크 · Rust 리소스 추적 | 01a1b63c-3b93-7371-b055-9fc2cec587b8 | Read-only review of 14 legacy watermark responsibilities and resource-watermark SoT/plan/evidence; misc lane when free, no crate/graph/code before root design acceptance |
| Humanymous · 모델 · 학습과 보정 이관 | 01a1b63c-d8c7-76c1-bc35-baf9d6cfe88c | Read-only legacy model training/correction and current Rust model gap review; model-training-correction SoT/plan/evidence on free sot-plan lane; no code/graph, activation, GPU, download or inference before root acceptance and allocation; separate from skill paired evaluation |
| Humanymous · 품질 · 컨벤션과 구조 감사 | 01a1b645-0bdc-7a53-b759-57c1eb998336 | Read-only convention/structure audit and measurement proposals; convention-audit SoT/plan/evidence only after actual dedicated lane acquisition; no foreign code, Pages, rules or graph writes; no Cargo/load tests before allocation |
| Humanymous · 성능 · 빌드와 런타임 최적화 | 01a1b645-0e76-75b0-b509-fd8e7719fc51 | Read-only build/runtime performance research and actual measurement plan; performance-optimization SoT/plan/evidence only after actual dedicated lane acquisition; future Rust benchmarks need a separate path grant and execution allocation |

Root owns shared Cargo/lock/toolchain, program roadmap and final Git integration. Pages owner alone writes docs/**, including public Draft v0. Other owners submit INTEGRATION.md/spec deltas. Source authoring may parallelize when real paths do not overlap.

Temporary performance coordination: root lent the existing project-coordination
lane to codex/rust-performance-baseline, observed with the exact four performance
spec/evidence paths recorded in LANES. The owner actually acquired it through the
unchanged PS compatibility protocol. This is not a new Rust-only lane grant or
permission to edit the lane's usual agent metadata. Convention audit remains
read-only pending separate ownership; future code/benchmarks require root scope
and execution assignment.

Local validation has at most four root-assigned pipelines A/B/C/D, jobs1/incremental0 and distinct targets, never same-target overlap or implicit permission to start. Admit only at commit<85% and free physical>20GiB. At commit>=88% or free physical<16GiB defer new compilation and automatically reduce subsequent allocations. Native Docker/CEF/SDK heavy builds are exclusive against other cold compilation streams; four slots do not prove concurrent cold capacity. Warm unit/bounded process overlap needs explicit budget admission. Deferred heavy builds wait for all other cold streams to become terminal and admission guards to pass. Preserve caches; no WSL shutdown, global cache drop, Docker pruning or foreign kills for admission. GPU inference remains separately allocated. Resource measurements and manifest windows follow program-roadmap/delivery and rule94.

Supported Windows/WSL compilation and ordinary process/IPC/fixture tests may run locally by root allocation. Selected Linux pre-push checks may use the same YAML in Windows Desktop act under rule96. GitHub retains unavailable OS/architecture and SCM/account profiles, event/permission semantics, genuine hosted contracts and the final matrix. Preserve actual venue and source-bound evidence; never relabel local as hosted. Rule94 records the measured basis and its limits. User-authorized model smoke remains separately bounded and does not qualify full paired evaluation. No publication or paid infrastructure is authorized. Only explicitly accepted bounded language/unsafe exceptions in controlling SoTs apply.

