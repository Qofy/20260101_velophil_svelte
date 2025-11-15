// API client for backend communication

const API_BASE = import.meta.env.VITE_API_BASE_URL || '/api';

export async function pingServer(): Promise<boolean> {
  try {
    const response = await fetch('/health');
    return response.ok;
  } catch {
    return false;
  }
}

export async function authLogin(email: string, password: string): Promise<any> {
  const response = await fetch(`${API_BASE}/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email, password }),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Login failed: ${response.status} ${errorText}`);
  }

  return response.json();
}

export async function authRegister(email: string, password: string): Promise<any> {
  const response = await fetch(`${API_BASE}/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email, password }),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Registration failed: ${response.status} ${errorText}`);
  }

  return response.json();
}

export async function authLogout(): Promise<void> {
  await fetch(`${API_BASE}/auth/logout`, {
    method: 'POST',
    credentials: 'include',
  });
}

export async function authMe(): Promise<any> {
  const response = await fetch(`${API_BASE}/auth/me`, {
    credentials: 'include',
  });

  if (!response.ok) {
    throw new Error('Not authenticated');
  }

  return response.json();
}

export async function saveOrder(order: any): Promise<any> {
  const response = await fetch(`${API_BASE}/orders`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(order),
  });
  if (!response.ok) throw new Error('Failed to save order');
  return response.json();
}

export async function saveReservation(reservation: any): Promise<any> {
  const response = await fetch(`${API_BASE}/reservations`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(reservation),
  });
  if (!response.ok) throw new Error('Failed to save reservation');
  return response.json();
}

export async function postKitchenPrint(job: any): Promise<any> {
  const response = await fetch(`${API_BASE}/kitchen/print`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(job),
  });
  if (!response.ok) throw new Error('Failed to print kitchen job');
  return response.json();
}

export async function kitchenTestPrint(test: any): Promise<any> {
  const response = await fetch(`${API_BASE}/kitchen/test`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(test),
  });
  if (!response.ok) throw new Error('Failed to test kitchen print');
  return response.json();
}

export async function listRuntimeCounts(): Promise<any> {
  const response = await fetch(`${API_BASE}/runtime/counts`, {
    credentials: 'include',
  });
  if (!response.ok) throw new Error('Failed to fetch runtime counts');
  return response.json();
}
