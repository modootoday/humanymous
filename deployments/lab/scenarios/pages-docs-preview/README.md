# Pages / docs preview (pointer only)

Owned Pages preview historically used project name `humanymous-pages-preview`
(localhost). This suite **must not** reuse that project name.

Pages qualification is docs/tooling (`humanymous-pages`), not a product Gate
compose stack. No compose.yaml is authored here to avoid colliding with an
already-running preview container. Operators: stop or rename any existing
`humanymous-pages-preview` before ad-hoc preview work; product gates remain
under `scenarios/*` and `adapters/*`.

Gate feed: none (docs validation / Pages SoT — not rust-product-lab e2e).
