import { afterEach, describe, expect, test, vi } from "vitest";
import { api } from "../api";

afterEach(() => {
  vi.unstubAllGlobals();
});

function mockFetch(impl: typeof fetch): void {
  vi.stubGlobal("fetch", vi.fn(impl));
}

describe("api client", () => {
  test("crewLeads parses 200 JSON array", async () => {
    mockFetch(
      async () =>
        new Response(JSON.stringify([{ id: "cl-aria", name: "A" }]), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
    );
    const r = await api.crewLeads();
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.value[0]!.id).toBe("cl-aria");
  });

  test("maps known DomainError code from response body", async () => {
    mockFetch(
      async () =>
        new Response(
          JSON.stringify({ error: "x", code: "PassengerNotFound" }),
          {
            status: 404,
          },
        ),
    );
    const r = await api.passenger("ps-zzz");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe("PassengerNotFound");
  });

  test("network failure surfaces NetworkError", async () => {
    mockFetch(async () => {
      throw new Error("offline");
    });
    const r = await api.passengers();
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe("NetworkError");
  });

  test("204 No Content returns ok(undefined)", async () => {
    mockFetch(async () => new Response(null, { status: 204 }));
    const r = await api.reset("cl-aria");
    expect(r.ok).toBe(true);
  });

  test("reset sends actor_id in body", async () => {
    const seen: { url?: string; init?: RequestInit | undefined } = {};
    mockFetch(async (input, init) => {
      seen.url = String(input);
      if (init !== undefined) seen.init = init;
      return new Response(null, { status: 204 });
    });
    await api.reset("cl-aria");
    expect(seen.url).toMatch(/\/reset$/);
    expect(seen.init?.method).toBe("POST");
    expect(JSON.parse(String(seen.init?.body))).toEqual({
      actor_id: "cl-aria",
    });
  });
});
