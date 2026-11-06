import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { Engine, Task, TaskRequest } from './types';
export const desktop = isTauri();
export async function tasks(): Promise<Task[]> { return desktop ? invoke('list_tasks') : []; }
export async function engines(): Promise<Engine[]> { return desktop ? invoke('list_engines') : ['qpdf','ffmpeg','imagemagick'].map(id=>({id,installed:false,version:null,available:false})); }
export async function enqueue(request:TaskRequest):Promise<void> { if(!desktop) throw new Error('Ouvrez l’application de bureau pour traiter vos fichiers.'); await invoke('enqueue_task',{request}); }
export async function cancel(id:string) { await invoke('cancel_task',{id}); }
export async function clearHistory() { if(desktop) await invoke('clear_history'); }
export async function reveal(path:string) { await invoke('open_result',{path}); }
export async function install(id:string) { await invoke('install_engine',{id}); }
export async function pickFiles(multiple:boolean):Promise<string[]> { if(!desktop) throw new Error('La sélection de fichiers est disponible dans l’application de bureau.'); const files=await open({multiple,directory:false}); return files ? (Array.isArray(files)?files:[files]) : []; }
export async function pickDirectory():Promise<string|null> { return open({directory:true,multiple:false}); }

export async function stopAll() { await invoke("stop_all"); }
