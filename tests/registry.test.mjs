import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { tools, selectTools } from '../src/tools.mjs';
const registry=JSON.parse(readFileSync(new URL('../src-tauri/catalog/modules.json',import.meta.url)));
test('sept opérations uniques et contrats de modules concordants',()=>{
 assert.equal(tools.length,7);assert.equal(new Set(tools.map(t=>t.id)).size,7);
 assert.equal(registry.contractVersion,1);
 assert.deepEqual([...registry.modules.flatMap(m=>m.operations)].sort(),tools.map(t=>t.id).sort());
});
test('recherche française sans accents',()=>{
 assert.equal(selectTools('Tous les outils','video',[]).length,2);
 assert.equal(selectTools('PDF','fusionner',[])[0].id,'pdf.merge');
 assert.equal(selectTools('PDF','audio',[]).length,0);
});
test('favoris filtrés sans modifier le registre',()=>{
 assert.deepEqual(selectTools('Favoris','',['audio.convert']).map(t=>t.id),['audio.convert']);
 assert.equal(selectTools('Favoris','',[]).length,0);assert.equal(tools.length,7);
});
test('aucun moteur non qualifié n’est proposé',()=>{
 const catalog=JSON.parse(readFileSync(new URL('../src-tauri/catalog/engines.json',import.meta.url)));
 assert.equal(catalog.schemaVersion,1);
 for(const a of catalog.artifacts){assert.match(a.url,/^https:\/\/github\.com\/Hicha-m\/tool-kit\/releases\/download\//);assert.match(a.sha256,/^[a-f0-9]{64}$/);assert.ok(a.size>0);assert.ok(a.unpackedLimit>0);assert.ok(a.license);}
});
