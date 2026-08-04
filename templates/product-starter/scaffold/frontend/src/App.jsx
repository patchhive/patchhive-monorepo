import { useCallback, useEffect, useMemo, useState } from "react";
import { CircleDot, Database, ShieldCheck } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  ProductHeader,
  ProductLoginScreen,
  ProductShell,
  StartupCheckList,
  V3_TEXT,
} from "@patchhivehq/ui";
import { API } from "./config.js";

const config = {
  icon: CircleDot,
  name: "__PRODUCT_TITLE__",
  productKey: "__PRODUCT_SLUG__",
};

function Fact({ label, value }) {
  return <div className="surface-inset rounded-xl p-4"><div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div><div className={`mt-2 font-display text-[20px] font-semibold ${V3_TEXT.strong}`}>{value ?? "—"}</div></div>;
}

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "__PRODUCT_SLUG___api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const [activeTab, setActiveTab] = useState("workspace");
  const [health, setHealth] = useState(null);
  const [checks, setChecks] = useState([]);
  const [overview, setOverview] = useState(null);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    setError("");
    try {
      const [healthResponse, checksResponse, overviewResponse] = await Promise.all([
        fetcher(`${API}/health`),
        fetcher(`${API}/startup/checks`),
        fetcher(`${API}/overview`),
      ]);
      if (!healthResponse.ok || !checksResponse.ok || !overviewResponse.ok) throw new Error("The starter backend returned an error.");
      setHealth(await healthResponse.json());
      setChecks((await checksResponse.json()).checks || []);
      setOverview(await overviewResponse.json());
    } catch (requestError) {
      setError(requestError.message || "Cannot reach the backend.");
    }
  }, [fetcher]);

  useEffect(() => {
    if (!auth.needsAuth && auth.checked) void refresh();
  }, [auth.checked, auth.needsAuth, refresh]);

  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`grid min-h-screen place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen auth={auth} config={config} />;

  const tabs = [{ id: "workspace", label: "Workspace" }, { id: "checks", label: "Checks" }, { id: "sources", label: "Sources" }];
  return <ProductShell productKey={config.productKey}>
    <ProductHeader activeTab={activeTab} githubLabel="Local starter" icon={CircleDot} onRun={refresh} onSignOut={auth.logout} onTabChange={setActiveTab} productName="__PRODUCT_TITLE__" runLabel="Refresh" subtitle="__PRODUCT_TAGLINE__" tabs={tabs} />
    <div className="mx-auto max-w-[1440px] px-6 py-8">
      {error ? <div className="surface mb-6 border border-red-500/30 p-4 text-[12px] text-red-700 dark:text-red-300">{error}</div> : null}
      {activeTab === "workspace" ? <section className="grid grid-cols-12 gap-6"><article className="surface col-span-12 p-8 lg:col-span-8"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Product workspace</div><h1 className={`mt-3 font-display text-[46px] font-semibold ${V3_TEXT.strong}`}>Build the real product loop here.</h1><p className={`mt-4 max-w-2xl text-[14px] leading-relaxed ${V3_TEXT.body}`}>{overview?.message || "The starter is connected to the unified PatchHive architecture and ready for product-owned behavior."}</p></article><aside className="surface col-span-12 p-6 lg:col-span-4"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Runtime</div><div className="mt-4 grid grid-cols-2 gap-3"><Fact label="Backend" value={health?.status || "loading"} /><Fact label="Database" value={health?.db_ok ? "ready" : "unknown"} /></div></aside></section> : null}
      {activeTab === "checks" ? <section className="grid grid-cols-12 gap-6"><article className="surface col-span-12 p-6 lg:col-span-8"><div className="mb-5 flex items-center gap-2"><ShieldCheck size={14} /><h1 className={`font-display text-[28px] font-semibold ${V3_TEXT.strong}`}>System checks.</h1></div><StartupCheckList checks={checks} /></article><aside className="surface col-span-12 p-6 lg:col-span-4"><div className="flex items-center gap-2"><Database size={13} /><span className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Product state</span></div><div className="mt-4 space-y-3"><Fact label="Status" value={health?.status} /><Fact label="Database path" value={health?.db_path} /></div></aside></section> : null}
      {activeTab === "sources" ? <section className="surface p-8"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Product intake</div><h1 className={`mt-3 font-display text-[38px] font-semibold ${V3_TEXT.strong}`}>Define an honest input scope.</h1><p className={`mt-4 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>Replace this starter copy with the product's direct and discovery inputs, connection requirements, and safety boundary. Do not leave placeholder routes or controls after the first real workflow lands.</p></section> : null}
    </div>
  </ProductShell>;
}
