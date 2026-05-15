export const SORT_OPTIONS = [
  { v: "priority", l: "Priority" },
  { v: "stale", l: "Most Stale" },
  { v: "todos", l: "Most TODOs" },
  { v: "name", l: "Name A–Z" },
  { v: "stars", l: "Stars" },
];

export function sortRepos(repos, sortBy) {
  if (!repos) return [];
  const list = [...repos];
  switch (sortBy) {
    case "stale":
      return list.sort((a, b) => (b.stale_issues || 0) - (a.stale_issues || 0));
    case "todos":
      return list.sort((a, b) => (b.todo_count || 0) - (a.todo_count || 0));
    case "name":
      return list.sort((a, b) => a.full_name.localeCompare(b.full_name));
    case "stars":
      return list.sort((a, b) => (b.stars || 0) - (a.stars || 0));
    default:
      return list.sort((a, b) => (b.priority_score || 0) - (a.priority_score || 0));
  }
}
