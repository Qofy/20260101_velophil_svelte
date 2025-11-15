// Data utilities and helpers

export function completeTable(items: any[]): any[] {
  // Complete table data with defaults
  return items.map((item, index) => ({
    id: item.id || index,
    ...item,
  }));
}
