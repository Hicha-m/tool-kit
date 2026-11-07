export interface Tool { id: string; category: string; engine: string; title: string; description: string; glyph: string; output: string; multiple?: boolean }
export interface TaskRequest { operation: string; inputs: string[]; destination: string; format: string; pages: string; width: number; quality: number; track: number }
export interface Task { id: string; operation: string; status: 'queued'|'running'|'succeeded'|'failed'|'cancelled'|'interrupted'; created: number; output: string|null; error: string|null }
export interface Engine { id: string; installed: boolean; version: string|null; available: boolean }
