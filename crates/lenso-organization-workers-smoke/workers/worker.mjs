import * as generated from "./pkg/lenso_organization_workers_smoke.js";
import wasmModule from "./pkg/lenso_organization_workers_smoke_bg.wasm";
import { createWorkersHttpHost, createEventScope } from "@lenso/workers-runtime";
import { createD1Binding } from "./binding.mjs";

export default createWorkersHttpHost({
  bindings: {
    ...generated,
    async handle_http(_input, scope) {
      const outcome = scope.mode === "setup"
        ? (await generated.migrate(scope.batch), "setup")
        : await generated.exercise(scope.batch, scope.mode, scope.id, scope.close);
      return JSON.stringify({
        status: 200, headers: [],
        body: Array.from(new TextEncoder().encode(JSON.stringify({ outcome }))),
        shutdown: "clean",
      });
    },
  },
  wasmModule,
  limits: { eventLimitMs: 30000 },
  createScope(request, env) {
    const url = new URL(request.url);
    const mode = url.searchParams.get("mode") || "success";
    const id = url.searchParams.get("id") || mode;
    return createEventScope((resources) => {
      const binding = createD1Binding(mode === "missing-schema" ? env.EMPTY : env.ORGANIZATION, resources);
      return {
        mode, id, close: () => resources.abort(),
        batch: mode === "throws" ? () => { throw new Error("synthetic-private-data"); }
          : mode === "malformed" ? async () => JSON.stringify([{ success: false, results: [] }])
          : mode === "rollback" ? (input) => {
              const statements = JSON.parse(input);
              if (statements.some((s) => s.sql.startsWith("INSERT INTO organization_operation"))) {
                // Fail after the membership and receipt writes, inside this same D1 batch.
                statements.push({ sql: "SELECT * FROM injected_missing_table", params: [] });
              }
              return binding(JSON.stringify(statements));
            }
          : binding,
      };
    });
  },
});
