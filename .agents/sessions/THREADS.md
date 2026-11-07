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
| Humanymous · Windows · 네이티브 러너 | 01a1b30d-439d-7030-8e38-789bc0b70f6a | Existing owned context; preserve current claims and plans |
| Humanymous · CLI · 진단과 설치 | 01a1b2f5-e927-7a03-9d47-9d1084d14c53 | Existing owned context; preserve current claims and plans |
| Humanymous · 브라우저 · Meari와 CEF | 01a1b35f-d3e9-7042-91a4-10c910a15515 | Existing owned context; preserve current claims and plans |
| Humanymous · 감사 · 키 보관과 영속성 | 01a1b35f-d1a1-7553-a0ed-65d09b9042a7 | Existing owned context; preserve current claims and plans |
| Humanymous · 어댑터 · 서버 프레임워크 | 01a1b2da-d1e9-7a22-9052-9a90a8cba54f | Existing owned context; preserve current claims and plans |
| Humanymous · 제품랩 · Docker와 브라우저 격리 | 01a1b30d-4736-7281-a54f-5cb3a5fa4fe4 | Existing owned context; preserve current claims and plans |
| Humanymous · CI/CD · Cross-OS와 OIDC | 01a1b2da-7d8a-73f2-9bb2-78bde5ae33f9 | Existing owned context; preserve current claims and plans |
| Humanymous · Apache · httpd 모듈 | 01a1b4e3-b0f1-7320-81a8-13e89d1d0dff | adapters/apache/**,crates/apache-adapter/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · 네이티브 ABI · C와 C++ 연결 | 01a1b4e3-b2b7-71d3-bf3b-66a851cb7cf4 | crates/native-abi/**,adapters/native/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · Go · HTTP 프레임워크 연결 | 01a1b4e3-b4a2-7690-9057-24b06d04b652 | crates/go-adapter/**,adapters/go/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · .NET · ASP.NET Core 연결 | 01a1b4e3-b687-7822-88c3-20d053f7d8fd | crates/dotnet-adapter/**,adapters/dotnet/**; dedicated source/spec/pilot goal dispatched |
| Humanymous · 네트워크 · TLS와 HTTP2 캡처 | 01a1b4e3-b852-7d21-90d9-4f00bebb5420 | crates/capture/**; dedicated source/spec/pilot goal dispatched |

Root owns shared Cargo/lock/toolchain, program roadmap and final Git integration. Pages owner alone writes docs/**, including public Draft v0. Other owners submit INTEGRATION.md/spec deltas. Local Cargo is globally serial; source authoring may parallelize without a count cap when real paths do not overlap. No publication or paid infrastructure is authorized. Foreign language glue/unsafe exceptions remain unresolved and must not be silently introduced.

