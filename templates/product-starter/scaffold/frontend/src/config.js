const defaultApiBase =
  typeof window !== "undefined" && !import.meta.env.DEV && window.location?.origin
    ? `${window.location.origin}/api/products/__PRODUCT_SLUG__`
    : "http://localhost:__BACKEND_PORT__";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;
