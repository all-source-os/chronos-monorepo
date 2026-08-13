[
  # JOSE library returns {true/false, JWT, JWS}, not {:error, reason}
  # These defensive pattern matches are unreachable per dialyzer
  ~r/user_socket\.ex.*pattern_match/,
  ~r/auth_pipeline\.ex.*pattern_match/,
  ~r/jwt_auth\.ex.*pattern_match/,
  # Pattern match coverage warnings (exhaustive matches flagged by dialyzer)
  ~r/query_controller\.ex.*pattern_match_cov/,
  ~r/tenant_context\.ex.*pattern_match_cov/,
  # The two filters below look redundant, and exactly one of them is — but
  # *which* one depends on the toolchain, so both have to stay.
  # `mix dialyzer --list-unused-filters` reports:
  #   OTP 27 (CI):    `pattern_match` unused, `call_with_opaque` used
  #   OTP 28 (local): `call_with_opaque` unused, `pattern_match` used
  # The two OTPs disagree about which warning Mint's opaque conn provokes.
  # Deleting either one greens the toolchain that reported it unused and reds
  # the other, so keep both until CI and local run the same OTP.
  #
  # Mint.WebSocket.new can return {:ok, conn, ws} but Dialyzer infers only error from typespecs
  ~r/core_websocket_worker\.ex.*pattern_match/,
  # `Mint.WebSocket.upgrade/4` takes the `Mint.HTTP.t()` handed back by
  # `Mint.HTTP.connect/4` — exactly what the library documents. Newer
  # mint_web_socket declares that parameter against a tightened opaque type, so
  # Dialyzer reports "call with opaque term" for correct usage: the opaque
  # struct crosses a module boundary and it cannot see inside. There is no
  # source change that fixes this without abandoning Mint's public API.
  ~r/core_websocket_worker\.ex.*call_with_opaque/
]
