# grpc-widgets-example

Smallest possible `transport grpc` CrateStack server — one `Widget` model, CRUD over real gRPC (HTTP/2, h2c plaintext) — plus two generated client integrations calling it live: TypeScript (browser gRPC-Web) and Dart (native `package:grpc`).

## What it shows

- **`transport grpc`, end to end.** The schema (`schemas/widgets.cstack`) declares `transport grpc` instead of REST/RPC; `include_server_schema!` mounts a real tonic service (`ModelWidgetList/Get/Create/Update/Delete` on `widgets_api.Api`) via `cratestack_schema::grpc::into_router` — no external proxy, one deployable binary.
- **The `.proto` contract and field-number lockfile.** `schemas/widgets.pb.lock` (committed) pins every field's wire number; `schemas/widgets.proto` is the artifact `cratestack generate-proto` emits from it — the same numbers the Rust server, the TypeScript client, and the Dart client all agree on. See [`docs/design/protobuf.md`](../../docs/design/protobuf.md).
- **Two independently-generated clients, same schema, same server.** `ts-client/` talks gRPC-Web over `fetch` (browser-viable, no external proxy — an in-process `tonic-web` layer handles it). `dart-client/` talks real HTTP/2 gRPC via `package:grpc`. Neither depends on `protoc`, `protoc-gen-grpc-web`, or `protoc-gen-dart` — both wire codecs are hand-rolled by their respective CrateStack generators.
- **`grpcurl` works too**, since it's a real gRPC service — see the server's own `cargo run` output for the exact command.

## Layout

| Path | What |
|---|---|
| [`schemas/widgets.cstack`](schemas/widgets.cstack) | The schema: one `Widget` model, `transport grpc` |
| [`schemas/widgets.pb.lock`](schemas/widgets.pb.lock) | Committed field-number lockfile (`docs/design/protobuf.md` §3.3) |
| [`schemas/widgets.proto`](schemas/widgets.proto) | Generated `.proto` artifact (`cratestack generate-proto`) |
| [`src/main.rs`](src/main.rs) | The server: `include_server_schema!`, header-driven auth, `grpc::into_router` |
| [`ts-client/`](ts-client) + [`ts-client-e2e.mjs`](ts-client-e2e.mjs) | Generated TypeScript gRPC-Web client + live integration test |
| [`dart-client/`](dart-client) + [`dart-client/tool/e2e.dart`](dart-client/tool/e2e.dart) | Generated Dart gRPC client + live integration test |

## Run the server

```bash
export DATABASE_URL=postgres://cratestack:cratestack@localhost:55432/cratestack_test
cargo run -p grpc-widgets-example
```

(`compose.yml` at the repo root brings up that Postgres — `docker compose up -d postgres`.)

Verify with `grpcurl` (no client generation needed):

```bash
grpcurl -plaintext -import-path examples/grpc-widgets/schemas -proto widgets.proto \
  -H 'x-auth-id: 1' -d '{"name": "gizmo"}' \
  localhost:50061 widgets_api.Api/ModelWidgetCreate
```

## Run the TypeScript client (gRPC-Web)

```bash
cd examples/grpc-widgets/ts-client && npm install && npm run build
node examples/grpc-widgets/ts-client-e2e.mjs
```

Drives the client through `fetch` against the running server: CORS trailer-header exposure, create/get/list/update/delete, and a typed `CratestackGrpcError` on get-after-delete. See [`ts-client/README.md`](ts-client/README.md) for the generated package's own API docs.

## Run the Dart client (native gRPC)

```bash
cd examples/grpc-widgets/dart-client && flutter pub get
dart run tool/e2e.dart
```

Same checks as the TypeScript script, over real HTTP/2 instead of gRPC-Web — `package:grpc`'s `ClientChannel` + `ChannelCredentials.insecure()` (plaintext, matching the server) and a hand-rolled protobuf wire codec (varint/fixed64/length-delimited) driving `ClientMethod`'s serializer seam directly, no `protoc-gen-dart` step. See [`dart-client/README.md`](dart-client/README.md) for the generated package's own API docs, including per-call `CallOptions` and closing the connection (`runtime.shutdown()`).

## Scope

Model CRUD only — procedures and server-streaming aren't wired into the generated tonic service yet (`cratestack-grpc`'s own scope note). `list()` on both clients supports `limit`/`offset`/`fields`/`include`/`sort`; raw predicate queries (`where`/`or`) and structured filters aren't wired up in either generator yet.

## See Also

- [`docs/design/protobuf.md`](../../docs/design/protobuf.md) — the design doc this example implements
- Epic [#166](https://github.com/cratestack/cratestack/issues/166), tickets #168–#173 (Rust/proto/gRPC-Web), [#210](https://github.com/cratestack/cratestack/issues/210) (Dart gRPC client)
- [`examples/rpc-streaming-client-rust/`](../rpc-streaming-client-rust/) — a REST/RPC-transport client/server pair, for comparison
