// Svelte stores for reactive state management
import { writable, derived, type Writable } from 'svelte/store';

// Backend connectivity
export const backendOnline = writable(false);
export const backendMessageStore = writable('');
export const pendingOpsCount = writable(0);

// Kitchen configuration
export const kitchenUseRelay = writable(false);
export const kitchenUseTest = writable(false);
export const kitchenTestLatencyMs = writable(100);
export const kitchenTestFailRate = writable(0);

// Runtime data
export const runtimeEventsCursor = writable(0);
export const runtimeOrders = writable<any[]>([]);
export const runtimeReservations = writable<any[]>([]);
export const runtimePayments = writable<any[]>([]);
export const kitchenJobs = writable<any[]>([]);
export const runtimeCounts = writable<any>({});
export const runtimeCountsWindowMinutes = writable(60);

// Controls configuration
export interface ControlsConfig {
  layout?: string;
  [key: string]: any;
}

export const controlsConfig: Writable<ControlsConfig> = writable({});
export const conflictPolicy = writable('keep-remote');

// User authentication
export const currentUser = writable<any>(null);
export const isFirstUserSystem = writable(false);
