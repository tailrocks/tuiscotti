# State digest projections

The state digest streams borrowed snapshot and render-contract values through
`serde_json` directly into its hash writer. Snapshot projection omits capture
metadata and preserves the byte order produced by the previous `Value`
projection, including `serde_json`'s active map-ordering behavior. The private
render-contract projection keeps declaration order, option presence, and the
original floating-point types. These serializers are internal digest
compatibility code; they do not define sample persistence.
