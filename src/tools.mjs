export const tools = [
 { id:'pdf.merge', category:'PDF', engine:'qpdf', title:'Fusionner des PDF', description:'Rassemblez vos documents dans l’ordre de votre choix.', glyph:'▤', output:'pdf', multiple:true },
 { id:'pdf.extract', category:'PDF', engine:'qpdf', title:'Extraire des pages', description:'Gardez uniquement les pages qui vous intéressent.', glyph:'▥', output:'pdf' },
 { id:'audio.convert', category:'Audio', engine:'ffmpeg', title:'Convertir un audio', description:'Votre musique et vos enregistrements, au bon format.', glyph:'♫', output:'mp3', multiple:true },
 { id:'video.convert', category:'Vidéo', engine:'ffmpeg', title:'Convertir une vidéo', description:'Un fichier MP4 facile à lire et à partager.', glyph:'▷', output:'mp4', multiple:true },
 { id:'video.audio', category:'Vidéo', engine:'ffmpeg', title:'Extraire l’audio', description:'Retrouvez la bande-son de vos vidéos.', glyph:'♬', output:'mp3', multiple:true },
 { id:'image.convert', category:'Images', engine:'imagemagick', title:'Convertir une image', description:'Passez de JPEG à PNG ou WebP en quelques clics.', glyph:'▧', output:'png', multiple:true },
 { id:'image.resize', category:'Images', engine:'imagemagick', title:'Redimensionner', description:'La bonne taille, sans déformer vos images.', glyph:'⤢', output:'png', multiple:true },
];
export function selectTools(category, query, favorites) {
 const normalized=query.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toLowerCase();
 return tools.filter(t => (category==='Tous les outils' || (category==='Favoris' ? favorites.includes(t.id) : t.category===category)) && `${t.title} ${t.description} ${t.category}`.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toLowerCase().includes(normalized));
}
