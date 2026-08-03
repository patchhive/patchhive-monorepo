export type DiffToken = { type: "eq" | "add" | "del"; text: string };

// Word-level LCS diff. Returns tokens preserving whitespace boundaries.
export function wordDiff(before: string, after: string): DiffToken[] {
  const a = tokenize(before);
  const b = tokenize(after);
  const n = a.length;
  const m = b.length;

  // Build LCS length table
  const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }

  const out: DiffToken[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      pushToken(out, "eq", a[i]);
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      pushToken(out, "del", a[i]);
      i++;
    } else {
      pushToken(out, "add", b[j]);
      j++;
    }
  }
  while (i < n) pushToken(out, "del", a[i++]);
  while (j < m) pushToken(out, "add", b[j++]);
  return out;
}

function tokenize(s: string): string[] {
  // Split into words + whitespace runs, preserving both.
  return s.match(/\s+|\S+/g) ?? [];
}

function pushToken(out: DiffToken[], type: DiffToken["type"], text: string) {
  const last = out[out.length - 1];
  if (last && last.type === type) last.text += text;
  else out.push({ type, text });
}
