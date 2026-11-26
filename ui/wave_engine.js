(function (root, factory) {
  const api = factory();
  if (typeof module === 'object' && module.exports) {
    module.exports = api;
  }
  if (root) {
    root.WaveEngine = api;
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
  'use strict';

  const WAVE_MOODS = ['all', 'energetic', 'calm', 'sad', 'favorites', 'new'];

  const GENERIC_GENRES = new Set([
    '', 'music', 'other', 'soundcloud', 'youtube', 'yandex', 'spotify',
    'unknown', 'audio', 'none', 'all'
  ]);

  const STOP_TOKENS = new Set([
    'the', 'and', 'for', 'with', 'from', 'this', 'that', 'you', 'your', 'are',
    'feat', 'prod', 'official', 'audio', 'track', 'song', 'music', 'version',
    'edit', 'mix', 'original', 'full', 'free', 'new', 'type', 'beat', 'beats',
    'что', 'это', 'как', 'для', 'или', 'без', 'под', 'про', 'при', 'мои', 'мой',
    'твоя', 'твой', 'меня', 'тебя', 'трек', 'песня', 'звук', 'сниппет'
  ]);

  const AI_GENERATOR_RE = /\b(?:suno(?:\.ai|\.com|ai)?|udio(?:\.ai|\.com|ai)?|soundraw|boomy|aimusic|aiva|mubert|riffusion|sonauto|producer\.ai|musicgen|audiocraft|stableaudio|stable audio|elevenlabs|loudly|beatoven|soundful|ace-step|acestep|kits\.ai|weights\.gg|chatgpt|openai|midjourney)\b/i;
  const AI_COVER_RE = /\b(?:rvc|diff-svc|so-vits(?:-svc)?|voice model|ai[- ]?cover|ai[- ]?version|ai[- ]?remake|ai[- ]?remix|ai[- ]?mashup|ai[- ]?duet|ai[- ]?feat)\b/i;
  const AI_EN_PHRASE_RE = /\b(?:ai[- ]?generated|generated (?:by|with) ai|created (?:by|with) ai|made (?:by|with) ai|powered by ai|text[- ]?to[- ]?music|ai[- ]?vocals?|ai[- ]?song|ai[- ]?music|ai[- ]?track|ai[- ]?rap|ai[- ]?beat|ai[- ]?phonk|ai[- ]?rock|ai[- ]?pop|ai[- ]?lo-?fi|ai[- ]?jazz|ai[- ]?metal|ai[- ]?artist|ai[- ]?voice|ai[- ]?slop)\b/i;
  const AI_PROMPT_LEAK_RE = /(?:made|created|generated)\s+(?:in|with|by|using)\s+(?:suno|udio|riffusion|ai)\b|(?:^|\n)\s*(?:style of music|prompt)\s*:/im;
  const AI_BRACKET_RE = /(?:^|[\s|/\\-])(?:\[(?:ai|ии)\]|[(](?:ai|ии)[)])(?:$|[\s|/\\-])/iu;
  const AI_RU_PHRASE_RE = /(?:^|[^\p{L}\p{N}])(?:нейросет\p{L}*|нейромузык\p{L}*|нейрокавер\p{L}*|нейро\s+кавер\p{L}*|нейротрек\p{L}*|нейро\s+трек\p{L}*|нейропесн\p{L}*|нейрохит\p{L}*|нейрорэп\p{L}*|нейрореп\p{L}*|нейрофонк\p{L}*|сгенерирован\p{L}*|создано\s+(?:ии|нейросет\p{L}*)|генерация\s+ии|искусственн\p{L}*\s+интеллект\p{L}*|ии[- ]?вокал|ии[- ]?кавер|ии[- ]?трек|ии[- ]?верси\p{L}*|ии[- ]?музык\p{L}*|кавер\s+ии)(?:$|[^\p{L}\p{N}])/iu;
  const AI_HASHTAG_RE = /(?:^|\s)#(?:suno|udio|sunoai|udioai|ai|aimusic|aigenerated|aicover|aivoice|aivocals|aisong|neural|нейросеть|нейромузыка|нейрокавер|нейротрек|ии|иикавер|иимузыка)\b/iu;
  const AI_GENRE_EXACT_RE = /^(?:ai|ai music|ai generated|suno|udio|нейросеть|ии)$/i;
  const AI_ARTIST_RE = /^(?:suno|udio|ai[-_](?:music|cover|song|track|gen|bot|slop)|.*[ _-]suno$|.*[ _-]udio$|suno[ _-].*|udio[ _-].*|ai\s*covers?|нейромузык.*|нейрокавер.*)/iu;
  const AI_HANDLE_RE = /(?:^|[-_])(?:suno|udio|aicover|aimusic|aigen|нейро)(?:$|[-_0-9])/iu;

  const SAD_STRONG_RE = /(?:^| )(?:sad(?:core|rap|ness)?|sorrow|melanchol(?:y|ic|ia)|emo|midwest emo|depress(?:ed|ing|ion|ive)?|heartbrok(?:en|e)|heartbreak|breakup|lonely|loneliness|alone|solitude|despair|tears|crying|cry|miss you|broken|grief|miserable|dead inside|hurt|empty|numb|regret|pain|gloom|slowcore|restless dreams|груст\p{L}*|печал\p{L}*|тоск\p{L}*|меланхол\p{L}*|одинок\p{L}*|слез\p{L}*|разбит\p{L}*|угаса\p{L}*|пустот\p{L}*|депресс\p{L}*|боль|ностальг\p{L}*)(?: |$)/u;
  const SAD_SOFT_RE = /(?:^| )(?:shoegaze|post punk|witch house|darkwave|coldwave|doomer|dsbm|cloud rap|slowed|reverb|nostalg\p{L}*|пост ?панк|лирик\p{L}*|думер\p{L}*|мрак|холод\p{L}*|осен\p{L}*|дожд\p{L}*)(?: |$)/u;

  const CALM_STRONG_RE = /(?:^| )(?:calm|relax(?:ing|ation)?|peaceful|ambient|chill(?:hop|out|wave)?|downtempo|lofi|lo fi|smooth jazz|jazz|soul|sleep|soft|gentle|quiet|serene|meditat\p{L}*|study|rain|dream pop|bedroom pop|ethereal|acoustic|piano|instrumental|vaporwave|trip hop|bossa|folk|classical|спокой\p{L}*|расслаб\p{L}*|уют\p{L}*|тихо|тишин\p{L}*|сон|медляк\p{L}*|атмосфер\p{L}*|лампов\p{L}*)(?: |$)/u;
  const CALM_SOFT_RE = /(?:^| )(?:slowed|reverb|slow|night|ноч\p{L}*)(?: |$)/u;

  const ENERGETIC_HARD_RE = /(?:^| )(?:phonk|drift phonk|dnb|drum and bass|breakcore|lolicore|hardcore|hardstyle|rave|techno|trance|edm|bass house|tech house|rage|drill|hoodtrap|hyperpop|glitchcore|jumpstyle|dubstep|riddim|cyberpunk|synthwave|metal|punk|pop punk|jersey club|sped up|speed up|nightcore|boosted|bass boosted|distortion|banger|workout|gym|бодр\p{L}*|энерг\p{L}*|кач\p{L}*|драйв\p{L}*|рейв\p{L}*|разгон\p{L}*|басс\p{L}*)(?: |$)/u;
  const ENERGETIC_SOFT_RE = /(?:^| )(?:dance|club|party|house|electro|rock|fast|hype|energy|fitness|running|дискотек\p{L}*|бас)(?: |$)/u;
  const HAPPY_RE = /(?:^| )(?:happy|upbeat|uplifting|joyful|cheerful|feel good|good vibes|positive|smile|sunny|sunshine|summer|disco|funk|funky|afrobeat|reggaeton|dancehall|tropical|bounce|groovy|groove|весел\p{L}*|радост\p{L}*|позитив\p{L}*|улыб\p{L}*|праздник\p{L}*|туса|тусов\p{L}*|вечеринк\p{L}*|кайф\p{L}*|танц\p{L}*|диско\p{L}*|солн\p{L}*|летн\p{L}*|лето)(?: |$)/u;

  const FALLBACK_MOOD_QUERIES = {
    energetic: ['drum and bass', 'phonk', 'electronic dance', 'breakcore', 'hyperpop'],
    calm: ['ambient chill', 'lofi chillhop', 'dream pop', 'acoustic indie', 'chillout'],
    sad: ['melancholy indie', 'emo rap', 'shoegaze', 'sad acoustic', 'dark wave'],
    new: ['indie electronic', 'underground hip hop', 'alternative', 'new wave'],
    all: ['electronic', 'indie alternative', 'hip hop', 'dream pop']
  };

  function normalizeWaveMood(mood) {
    return WAVE_MOODS.includes(mood) ? mood : 'all';
  }

  function waveMetadataText(value) {
    return String(value || '')
      .normalize('NFKC')
      .toLowerCase()
      .replace(/ё/g, 'е')
      .replace(/[^\p{L}\p{N}]+/gu, ' ')
      .trim();
  }

  function waveArtistKey(track) {
    return String(track?.artist || '').normalize('NFKC').trim().toLowerCase();
  }

  const EDIT_REUPLOAD_RE = /\b(?:slowed|sped\s*up|speed\s*up|reverb|nightcore|bass\s*boosted|best\s*part|loop|muffled|8d\s*audio)\b/i;
  const TITLE_NOISE_BRACKET_RE = /\s*[\[(][^)\]]*(?:slowed|sped\s*up|speed\s*up|reverb|nightcore|bass\s*boosted|best\s*part|loop|muffled|instrumental|original\s*version|alternate|unreleased|radio\s*version|extended|remaster|prod\.?|feat\.?|ft\.?|w\/|official|audio|video|lyrics|snippet)[^)\]]*[\])]/gi;
  const TITLE_NOISE_TAIL_RE = /\b(?:slowed\s*\+?\s*reverb|slowed\s+down|sped\s*up|speed\s*up|best\s*part(?:\s+loop)?|nightcore|bass\s*boosted|muffled|instrumental|original\s*version|radio\s*version|extended(?:\s+mix)?|reverb|prod\.?\s+by\b.*)$/i;

  function normalizeArtistToken(raw) {
    let clean = String(raw || '')
      .normalize('NFKC')
      .toLowerCase()
      .replace(/ё/g, 'е')
      .replace(/\.radio(?:\s*\d+)?/g, '')
      .replace(/\s*[\[(][^)\]]*[\])]/g, ' ')
      .replace(/#[\p{L}\p{N}_-]+/gu, ' ')
      .replace(/[^\p{L}\p{N}]+/gu, ' ')
      .replace(/\b(?:archive|official|topic)\b/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
    if (clean === 'tupac' || clean === '2pac shakur') clean = '2pac';
    return clean;
  }

  function stripSongTitleNoise(rawTitle) {
    const cleaned = String(rawTitle || '')
      .normalize('NFKC')
      .replace(/#[\p{L}\p{N}_-]+/gu, ' ')
      .replace(TITLE_NOISE_BRACKET_RE, ' ')
      .replace(/\b(?:feat\.?|ft\.?|prod\.?\s*by)\s+[^-—–]+$/i, ' ')
      .replace(TITLE_NOISE_TAIL_RE, ' ');
    return waveMetadataText(cleaned);
  }

  function splitTitleSegments(rawTitle) {
    return String(rawTitle || '')
      .split(/\s+[-—–]\s+/)
      .map(part => part.trim())
      .filter(Boolean);
  }

  function waveEffectiveArtists(track) {
    const keys = [];
    const add = (raw) => {
      const first = String(raw || '').split(/\s*(?:,|&|×|\bfeat\.?\b|\bft\.?\b|\bw\/)\s*/i)[0];
      const norm = normalizeArtistToken(first || raw);
      if (norm && !keys.includes(norm)) keys.push(norm);
    };
    add(track?.artist);
    const segments = splitTitleSegments(track?.title);
    if (segments.length >= 2) {
      const candidate = segments.length >= 3 ? segments[segments.length - 2] : segments[0];
      if (candidate && candidate.length <= 42) {
        add(candidate);
      }
    }
    return keys;
  }

  function waveSongKeys(track) {
    if (!track) return [];
    const keys = [];
    const addKey = (k) => {
      if (k && !keys.includes(k)) keys.push(k);
    };
    const uploader = normalizeArtistToken(track.artist);
    const rawTitle = String(track.title || '');
    const fullClean = stripSongTitleNoise(rawTitle);
    if (uploader && fullClean) {
      addKey(`${uploader}::${fullClean}`);
    }

    const segments = splitTitleSegments(rawTitle);
    let innerSong = fullClean;
    if (segments.length >= 2) {
      const segArtistRaw = segments[segments.length - 2].split(/\s*(?:,|&|×|\bfeat\.?\b|\bft\.?\b|\bw\/)\s*/i)[0];
      const segArtist = normalizeArtistToken(segArtistRaw);
      const segSong = stripSongTitleNoise(segments[segments.length - 1]);
      if (segArtist && segSong) {
        addKey(`${segArtist}::${segSong}`);
        innerSong = segSong;
      }
      if (segments.length >= 3) {
        const firstArtist = normalizeArtistToken(segments[0]);
        if (firstArtist && segSong) addKey(`${firstArtist}::${segSong}`);
      }
      // Handle "song - artist" titles like "sleepwalking - flawed mangoes (slowed)"
      const leftSong = stripSongTitleNoise(segments[0]);
      const rightArtist = normalizeArtistToken(segments[segments.length - 1].replace(TITLE_NOISE_BRACKET_RE, ''));
      if (leftSong && rightArtist && leftSong.length >= 4) {
        addKey(`${rightArtist}::${leftSong}`);
      }
    }

    if (EDIT_REUPLOAD_RE.test(rawTitle) || innerSong.length >= 18) {
      const baseTitle = innerSong || fullClean;
      if (baseTitle && baseTitle.length >= 9) {
        addKey(`song::${baseTitle}`);
      }
    }
    return keys;
  }

  function waveSongKey(track) {
    return waveSongKeys(track)[0] || '';
  }

  function collectSongKeys(tracks) {
    const keys = new Set();
    for (const track of (Array.isArray(tracks) ? tracks : [])) {
      for (const k of waveSongKeys(track)) {
        keys.add(k);
      }
    }
    return keys;
  }

  function hasMatchingSongKey(track, songKeySet) {
    if (!songKeySet || !songKeySet.size) return false;
    for (const k of waveSongKeys(track)) {
      if (songKeySet.has(k)) return true;
    }
    return false;
  }

  function addTrackSongKeys(track, songKeySet) {
    if (!songKeySet) return;
    for (const k of waveSongKeys(track)) {
      songKeySet.add(k);
    }
  }

  function parseTrackTags(tagList) {
    const raw = String(tagList || '');
    if (!raw.trim()) return [];
    const tags = [];
    const re = /"([^"]+)"|(\S+)/g;
    let match;
    while ((match = re.exec(raw)) !== null) {
      const cleaned = waveMetadataText((match[1] || match[2] || '').replace(/^#+/, ''));
      if (cleaned && !GENERIC_GENRES.has(cleaned)) {
        tags.push(cleaned);
      }
    }
    return tags;
  }

  function waveGenreKey(track) {
    const raw = waveMetadataText(String(track?.genre || '').replace(/^#+/, ''));
    if (!raw || GENERIC_GENRES.has(raw)) return '';
    if (/^(?:hip ?hop(?: rap)?|rap(?: hip ?hop)?)$/.test(raw)) return 'hip hop';
    if (/^(?:drum (?:and )?bass|dnb)$/.test(raw)) return 'drum and bass';
    if (/^(?:dance (?:and )?edm|edm)$/.test(raw)) return 'dance & edm';
    if (/^(?:r ?b(?: soul)?|rnb)$/.test(raw)) return 'r&b';
    if (/^(?:lo ?fi)$/.test(raw)) return 'lofi';
    if (/^(?:post ?punk)$/.test(raw)) return 'post-punk';
    if (/^(?:dream ?pop)$/.test(raw)) return 'dream pop';
    if (/^(?:witch ?house)$/.test(raw)) return 'witch house';
    if (/^(?:cloud ?rap)$/.test(raw)) return 'cloud rap';
    if (/^(?:deep ?house)$/.test(raw)) return 'deep house';
    if (/^(?:hyper ?pop)$/.test(raw)) return 'hyperpop';
    return raw;
  }

  function isAiSlopTrack(track, options) {
    if (!track || track.id == null) return true;
    const allowShort = Boolean(options && options.allowShort);
    if (!allowShort && track.duration_ms && track.duration_ms < 45000) return true;

    const primaryText = [
      track.title,
      track.artist,
      track.author,
      track.genre,
      track.tag_list,
      track.user_permalink,
      track.permalink_url
    ].filter(Boolean).join(' ').toLowerCase();

    const description = String(track.description || '').toLowerCase();
    const fullText = description ? `${primaryText}\n${description}` : primaryText;

    if (AI_GENERATOR_RE.test(fullText)) return true;
    if (AI_COVER_RE.test(fullText)) return true;
    if (AI_EN_PHRASE_RE.test(fullText)) return true;
    if (AI_PROMPT_LEAK_RE.test(fullText)) return true;
    if (AI_BRACKET_RE.test(primaryText)) return true;
    if (AI_RU_PHRASE_RE.test(fullText)) return true;
    if (AI_HASHTAG_RE.test(fullText)) return true;

    const genre = String(track.genre || '').trim().toLowerCase();
    if (genre && AI_GENRE_EXACT_RE.test(genre)) return true;

    const artist = String(track.artist || '').trim().toLowerCase();
    if (artist && AI_ARTIST_RE.test(artist)) return true;

    const userPermalink = String(track.user_permalink || '').trim().toLowerCase();
    if (userPermalink && AI_HANDLE_RE.test(userPermalink)) return true;

    return false;
  }

  function analyzeTrackMood(track) {
    if (!track || isAiSlopTrack(track, { allowShort: true })) {
      return { energetic: 0, calm: 0, sad: 0, happy: 0, hardEnergetic: false, primary: '' };
    }

    const genre = waveGenreKey(track);
    const tags = parseTrackTags(track.tag_list);
    const title = waveMetadataText(track.title);
    const structured = [genre, ...tags].filter(Boolean).join(' ');
    const combined = [structured, title].filter(Boolean).join(' ');

    let energetic = 0;
    let calm = 0;
    let sad = 0;
    let happy = 0;
    let hardEnergetic = false;

    if (SAD_STRONG_RE.test(structured)) sad += 2;
    else if (SAD_STRONG_RE.test(combined)) sad += 1.5;
    if (SAD_SOFT_RE.test(combined)) sad += 0.8;

    if (CALM_STRONG_RE.test(structured)) calm += 2;
    else if (CALM_STRONG_RE.test(combined)) calm += 1.5;
    if (CALM_SOFT_RE.test(combined)) calm += 0.8;

    if (ENERGETIC_HARD_RE.test(structured)) {
      energetic += 2;
      hardEnergetic = true;
    } else if (ENERGETIC_HARD_RE.test(combined)) {
      energetic += 1.8;
      hardEnergetic = true;
    }
    if (ENERGETIC_SOFT_RE.test(structured)) energetic += 1.5;
    else if (ENERGETIC_SOFT_RE.test(title)) energetic += 0.8;

    if (HAPPY_RE.test(structured)) happy += 2;
    else if (HAPPY_RE.test(combined)) happy += 1.5;

    if (happy > 0) energetic += 1;

    let primary = 'all';
    const top = Math.max(sad, calm, energetic, happy);
    if (top >= 1) {
      if (sad === top) primary = 'sad';
      else if (calm === top) primary = 'calm';
      else if (happy === top) primary = 'happy';
      else primary = 'energetic';
    }

    return { energetic, calm, sad, happy, hardEnergetic, primary };
  }

  function waveTrackMood(track) {
    return analyzeTrackMood(track).primary;
  }

  function waveMoodFit(track, mood) {
    if (!track || isAiSlopTrack(track, { allowShort: true })) return -1;
    const target = normalizeWaveMood(mood);
    if (target === 'all' || target === 'favorites' || target === 'new') return 0;

    const { energetic, calm, sad, happy, hardEnergetic } = analyzeTrackMood(track);

    if (target === 'energetic') {
      if (sad >= 0.8 || calm >= 0.8) return -1;
      if (energetic === 0 && happy === 0 && (calm > 0 || sad > 0)) return -1;
      if (energetic >= 2) return 2;
      if (energetic >= 1 || happy >= 1) return 1;
      return 0;
    }

    if (target === 'calm') {
      if (hardEnergetic) return -1;
      if (energetic >= 0.8 || happy >= 1.2) return -1;
      if (calm >= 2) return 2;
      if (calm >= 1) return 1;
      if (sad >= 1) return 0.5;
      return 0;
    }

    if (target === 'sad') {
      if (hardEnergetic || happy >= 0.8 || energetic >= 0.8) return -1;
      if (sad >= 2) return 2;
      if (sad >= 1) return 1;
      if (sad >= 0.8) return 0.8;
      if (calm >= 1.5) return 0.4;
      return 0;
    }

    return 0;
  }

  function extractTitleTokens(track) {
    const raw = waveMetadataText(track?.title);
    if (!raw) return [];
    const tokens = [];
    for (const part of raw.split(' ')) {
      if (part.length < 3 || /^\d+$/.test(part) || STOP_TOKENS.has(part)) continue;
      tokens.push(part);
    }
    return tokens;
  }

  function detectScriptBucket(track) {
    const text = `${track?.title || ''} ${track?.artist || ''}`;
    if (/[а-яё]/i.test(text)) return 'cyrillic';
    if (/[\u3040-\u30ff\u4e00-\u9faf\uac00-\ud7af]/.test(text)) return 'cjk';
    if (/[a-z]/i.test(text)) return 'latin';
    return '';
  }

  function addWeight(map, key, amount) {
    if (!key || !amount) return;
    map.set(key, (map.get(key) || 0) + amount);
  }

  function buildTasteProfile(likedTracks, recentlyPlayed, fallbackTracks) {
    const rawLikes = Array.isArray(likedTracks) ? likedTracks : [];
    const validLikes = [];
    const likedIds = new Set();

    for (const track of rawLikes) {
      if (!track || track.id == null || track.liked === false) continue;
      if (isAiSlopTrack(track, { allowShort: true })) continue;
      const sid = String(track.id);
      if (likedIds.has(sid)) continue;
      likedIds.add(sid);
      validLikes.push(track);
    }

    const recentHistory = Array.isArray(recentlyPlayed) ? recentlyPlayed : [];
    const recentTracks = recentHistory
      .map(entry => (entry && entry.track ? entry.track : entry))
      .filter(track => track && track.id != null && !isAiSlopTrack(track, { allowShort: true }));

    const recentWeightById = new Map();
    recentTracks.slice(0, 60).forEach((track, idx) => {
      const sid = String(track.id);
      if (!recentWeightById.has(sid)) {
        recentWeightById.set(sid, Math.max(0.15, 0.55 - idx * 0.008));
      }
    });

    let pool = validLikes;
    if (!pool.length) {
      const seenPool = new Set();
      const mergedFallback = [];
      for (const track of [...recentTracks, ...(Array.isArray(fallbackTracks) ? fallbackTracks : [])]) {
        if (!track || track.id == null || isAiSlopTrack(track, { allowShort: true })) continue;
        const sid = String(track.id);
        if (seenPool.has(sid)) continue;
        seenPool.add(sid);
        mergedFallback.push(track);
      }
      pool = mergedFallback;
    }

    const artistWeights = new Map();
    const userIdWeights = new Map();
    const genreWeights = new Map();
    const tagWeights = new Map();
    const titleTokenWeights = new Map();
    const scriptCounts = new Map();

    const applyTrackSignals = (track, weight) => {
      const artistKey = waveArtistKey(track);
      if (artistKey) {
        const entry = artistWeights.get(artistKey) || {
          artistKey,
          name: track.artist || artistKey,
          userId: track.user_id ?? null,
          weight: 0,
          count: 0
        };
        entry.weight += weight;
        entry.count += 1;
        if (entry.userId == null && track.user_id != null) entry.userId = track.user_id;
        artistWeights.set(artistKey, entry);
      }

      if (track.user_id != null) {
        addWeight(userIdWeights, String(track.user_id), weight);
      }

      const genre = waveGenreKey(track);
      if (genre) addWeight(genreWeights, genre, weight);

      for (const tag of parseTrackTags(track.tag_list)) {
        addWeight(tagWeights, tag, weight);
      }

      for (const token of extractTitleTokens(track)) {
        addWeight(titleTokenWeights, token, weight * 0.45);
      }

      const script = detectScriptBucket(track);
      if (script) addWeight(scriptCounts, script, weight);
    };

    const totalPool = Math.max(1, pool.length - 1);
    pool.forEach((track, idx) => {
      const recency = validLikes.length > 1 ? 1.0 - 0.45 * (idx / totalPool) : 1.0;
      const recentBoost = recentWeightById.get(String(track.id)) || 0;
      applyTrackSignals(track, recency + recentBoost);
    });

    // Also incorporate recent non-liked listens lightly so current session taste refines recommendations
    if (validLikes.length) {
      recentTracks.slice(0, 20).forEach((track, idx) => {
        if (!likedIds.has(String(track.id))) {
          applyTrackSignals(track, Math.max(0.15, 0.45 - idx * 0.02));
        }
      });
    }

    const maxGenre = Math.max(1, ...genreWeights.values());
    const normalizedGenres = new Map();
    for (const [k, v] of genreWeights.entries()) {
      normalizedGenres.set(k, v / maxGenre);
    }

    const maxTag = Math.max(1, ...tagWeights.values());
    const normalizedTags = new Map();
    for (const [k, v] of tagWeights.entries()) {
      normalizedTags.set(k, v / maxTag);
    }

    const totalScript = Array.from(scriptCounts.values()).reduce((sum, v) => sum + v, 0) || 1;
    const scriptShare = new Map();
    for (const [k, v] of scriptCounts.entries()) {
      scriptShare.set(k, v / totalScript);
    }

    const topArtists = Array.from(artistWeights.values())
      .sort((a, b) => b.weight - a.weight || b.count - a.count);

    const topGenres = Array.from(normalizedGenres.entries())
      .sort((a, b) => b[1] - a[1]);

    const topTags = Array.from(normalizedTags.entries())
      .sort((a, b) => b[1] - a[1]);

    return {
      hasLikes: validLikes.length > 0,
      likedTracks: validLikes,
      likedIds,
      pool,
      artistWeights,
      userIdWeights,
      genreWeights: normalizedGenres,
      tagWeights: normalizedTags,
      titleTokenWeights,
      scriptShare,
      topArtists,
      topGenres,
      topTags
    };
  }

  function isKnownLikedArtist(profile, track) {
    if (!profile || !track) return false;
    const artistKey = waveArtistKey(track);
    if (artistKey && profile.artistWeights?.has(artistKey)) return true;
    if (track.user_id != null && profile.userIdWeights?.has(String(track.user_id))) return true;
    return false;
  }

  function hashString(str) {
    let h = 2166136261;
    const s = String(str || '');
    for (let i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return h >>> 0;
  }

  function selectTasteSeeds(profile, mood, options = {}) {
    const target = normalizeWaveMood(mood);
    const count = Math.max(1, options.count || 5);
    const cursor = options.cursor || 0;
    const salt = options.salt || 0;
    const recentPlayedIds = options.recentPlayedIds || new Set();
    const recentSeedIds = options.recentSeedIds || new Set();
    const recentSeedArtists = options.recentSeedArtists || new Set();
    const queueTracks = Array.isArray(options.queueTracks) ? options.queueTracks : [];
    const maxQueueSeeds = Math.max(0, options.maxQueueSeeds ?? (queueTracks.length ? 1 : 0));

    const pool = profile?.pool || [];
    if (!pool.length) return [];

    const rawLikes = Array.isArray(profile?.likedTracks) ? profile.likedTracks : [];
    const fullLikes = rawLikes.filter(t => t && t.id != null && !isAiSlopTrack(t));
    const seedLikes = fullLikes.length ? fullLikes : rawLikes.filter(t => t && t.id != null && !isAiSlopTrack(t, { allowShort: true }));

    // When only favorites are requested, prioritize fresh favorites directly
    if (target === 'favorites') {
      const fresh = seedLikes.filter(t => !recentPlayedIds.has(String(t.id)));
      const source = fresh.length >= count ? fresh : seedLikes;
      return source.slice(0, count);
    }

    const targeted = target === 'energetic' || target === 'calm' || target === 'sad';
    const compatiblePool = pool.filter(t => !isAiSlopTrack(t) && waveMoodFit(t, target) >= 0);
    const moodMatchedPool = targeted ? compatiblePool.filter(t => waveMoodFit(t, target) > 0) : [];
    const preferredPool = moodMatchedPool.length >= 3 ? moodMatchedPool : compatiblePool;
    const nonSlopPool = preferredPool.length ? preferredPool : pool.filter(t => !isAiSlopTrack(t));
    const fallbackPool = pool.filter(t => !isAiSlopTrack(t, { allowShort: true }) && waveMoodFit(t, target) >= 0);
    const candidates = nonSlopPool.length ? nonSlopPool : (fallbackPool.length ? fallbackPool : pool.filter(t => !isAiSlopTrack(t, { allowShort: true })));

    const seedRecencyOrder = new Map();
    let seedPos = 0;
    for (const sid of recentSeedIds) {
      seedRecencyOrder.set(String(sid), seedPos++);
    }

    const total = Math.max(1, candidates.length - 1);
    const scored = candidates.map((track, idx) => {
      const sid = String(track.id);
      const fit = waveMoodFit(track, target);
      const genre = waveGenreKey(track);
      const artist = waveArtistKey(track);
      const artistCount = profile.artistWeights.get(artist)?.count || 0;
      const genreAffinity = profile.genreWeights.get(genre) || 0;
      const recency = target === 'new'
        ? 1.0 - 0.75 * (idx / total)
        : 1.0 - 0.3 * (idx / total);

      // Partition unused seeds strictly before recently-used ones so high mood/affinity scores don't lock seeds
      const usedSeed = recentSeedIds.has(sid);
      const usedArtist = Boolean(artist && recentSeedArtists.has(artist));
      const tier = usedSeed ? 2 : (usedArtist ? 1 : 0);
      const recencyRank = usedSeed ? (seedRecencyOrder.get(sid) ?? 0) : 0;

      let score = recency * 12 + genreAffinity * 8 - (artistCount >= 3 ? Math.min(6, artistCount - 2) : 0);
      if (targeted) {
        score += fit >= 2 ? 32 : fit >= 1 ? 20 : fit > 0 ? 10 : 0;
      }
      if (recentPlayedIds.has(sid)) score -= 6;
      // Rotate broadly across the whole liked library by cursor + session salt
      score += ((hashString(`${sid}:${cursor}:${salt}`) % 53) - 26) * 1.1;
      return { track, score, genre, artist, tier, recencyRank };
    });

    scored.sort((a, b) => a.tier - b.tier || a.recencyRank - b.recencyRank || b.score - a.score);

    const seeds = [];
    const chosenArtists = new Set();
    const chosenGenres = new Map();
    const chosenIds = new Set();

    // Pick compatible queue-tail seeds during wave continuation so the wave surfs forward cleanly
    const queueSeedCandidates = [];
    if (queueTracks.length && maxQueueSeeds > 0) {
      for (let i = queueTracks.length - 1; i >= 0; i--) {
        const qt = queueTracks[i];
        if (!qt || qt.id == null || isAiSlopTrack(qt)) continue;
        const sid = String(qt.id);
        const art = waveArtistKey(qt) || sid;
        if (recentSeedIds.has(sid) || chosenIds.has(sid) || chosenArtists.has(art)) continue;
        const qFit = waveMoodFit(qt, target);
        if (targeted ? qFit <= 0 : qFit < 0) continue;
        queueSeedCandidates.push(qt);
        chosenIds.add(sid);
        chosenArtists.add(art);
        if (queueSeedCandidates.length >= maxQueueSeeds) break;
      }
      chosenIds.clear();
      chosenArtists.clear();
    }

    const maxLikedSeeds = queueSeedCandidates.length
      ? Math.max(1, count - queueSeedCandidates.length)
      : count;

    // First pass: maximize artist AND genre diversity among liked seeds
    for (const item of scored) {
      if (seeds.length >= maxLikedSeeds) break;
      const sid = String(item.track.id);
      const art = item.artist || sid;
      if (chosenIds.has(sid) || chosenArtists.has(art)) continue;
      if (item.genre && (chosenGenres.get(item.genre) || 0) >= 2 && scored.some(other =>
        !chosenIds.has(String(other.track.id)) && !chosenArtists.has(other.artist || String(other.track.id))
        && (!other.genre || (chosenGenres.get(other.genre) || 0) < 2))) {
        continue;
      }
      chosenIds.add(sid);
      chosenArtists.add(art);
      if (item.genre) chosenGenres.set(item.genre, (chosenGenres.get(item.genre) || 0) + 1);
      seeds.push(item.track);
    }

    // Second pass if artist diversity couldn't fill count
    if (seeds.length < maxLikedSeeds) {
      for (const item of scored) {
        if (seeds.length >= maxLikedSeeds) break;
        const sid = String(item.track.id);
        if (chosenIds.has(sid)) continue;
        chosenIds.add(sid);
        seeds.push(item.track);
      }
    }

    // Append current-flow queue seeds
    for (const qt of queueSeedCandidates) {
      if (seeds.length >= count) break;
      const sid = String(qt.id);
      if (chosenIds.has(sid)) continue;
      chosenIds.add(sid);
      seeds.push(qt);
    }

    return seeds;
  }

  function buildTasteSearchQueries(profile, mood, cursor = 0) {
    const target = normalizeWaveMood(mood);
    if (target === 'favorites') return [];

    const genres = (profile?.topGenres || []).map(([g]) => g).filter(Boolean).slice(0, 4);
    const tags = (profile?.topTags || []).map(([t]) => t).filter(Boolean).slice(0, 4);
    const artists = (profile?.topArtists || []).map(a => a.name).filter(Boolean).slice(0, 4);

    const queries = [];
    const pushUnique = q => {
      const clean = String(q || '').trim();
      if (clean && !queries.includes(clean)) queries.push(clean);
    };

    const moodModifiers = {
      energetic: ['remix', 'bass', 'fast', 'club', 'sped up'],
      calm: ['chill', 'ambient', 'slowed', 'lofi', 'acoustic'],
      sad: ['sad', 'melancholy', 'slowed', 'emo'],
      new: [String(new Date().getFullYear()), String(new Date().getFullYear() - 1), 'premiere'],
      all: ['']
    }[target] || [''];

    const PURE_MOOD_WORDS = new Set(['sad', 'melancholy', 'calm', 'chill', 'energetic', 'happy', 'fast']);

    if (genres.length || tags.length || artists.length) {
      const styleTerms = [...new Set([...genres, ...tags])]
        .filter(term => !PURE_MOOD_WORDS.has(term));
      styleTerms.forEach((term, idx) => {
        const mod = moodModifiers[(cursor + idx) % moodModifiers.length];
        if (!mod || term.includes(mod) || mod.includes(term)) {
          pushUnique(term);
        } else {
          pushUnique(`${term} ${mod}`);
        }
      });
      if (!queries.length && artists.length) {
        artists.forEach((artist, idx) => {
          if (target === 'sad' || target === 'calm' || target === 'energetic') {
            const mod = moodModifiers[(cursor + idx) % moodModifiers.length];
            pushUnique(`${artist} ${mod}`);
          } else {
            pushUnique(artist);
          }
        });
      }
    }

    const ruMoodQueries = {
      energetic: ['бодрая музыка', 'драйв', 'рейв'],
      calm: ['спокойная музыка', 'эмбиент', 'чилл'],
      sad: ['грустная музыка', 'меланхолия', 'пост панк']
    };
    if ((profile?.scriptShare?.get('cyrillic') || 0) >= 0.2 && ruMoodQueries[target]) {
      for (const rq of ruMoodQueries[target]) pushUnique(rq);
    }

    if (!queries.length || target === 'sad' || target === 'calm' || target === 'energetic') {
      const fallback = FALLBACK_MOOD_QUERIES[target] || FALLBACK_MOOD_QUERIES.all;
      for (const fq of fallback) pushUnique(fq);
    }

    // Rotate starting position by cursor so consecutive calls vary queries
    const rotated = [];
    for (let i = 0; i < queries.length; i++) {
      rotated.push(queries[(cursor + i) % queries.length]);
    }
    return rotated.slice(0, 4);
  }

  function trackFreshnessScore(track) {
    if (track?.created_at) {
      const ts = Date.parse(track.created_at);
      if (Number.isFinite(ts)) {
        const ageDays = Math.max(0, (Date.now() - ts) / 86400000);
        if (ageDays <= 30) return 1.0;
        if (ageDays <= 90) return 0.82;
        if (ageDays <= 180) return 0.65;
        if (ageDays <= 365) return 0.45;
        if (ageDays <= 730) return 0.2;
        return -0.25;
      }
    }
    const numericId = Number(track?.id);
    if (Number.isFinite(numericId) && numericId > 0) {
      if (numericId >= 2200000000) return 0.85;
      if (numericId >= 1900000000) return 0.6;
      if (numericId >= 1500000000) return 0.3;
      if (numericId < 900000000) return -0.2;
    }
    return 0;
  }

  function rankWaveCandidates(candidateMap, profile, mood, options = {}) {
    const target = normalizeWaveMood(mood);
    const targeted = target === 'energetic' || target === 'calm' || target === 'sad';
    const seenIds = options.seenIds || new Set();
    const recentPlayedIds = options.recentPlayedIds || new Set();
    const recentServedIds = options.recentServedIds || new Set();
    const excludeServed = options.excludeServed !== false;
    const recentSongKeys = options.recentSongKeys || null;
    const seedArtists = options.seedArtists || new Set();
    const cursor = options.cursor || 0;
    const salt = options.salt || 0;
    const allowLiked = Boolean(options.allowLiked);
    const strictMood = Boolean(options.strictMood);

    const servedOrder = new Map();
    if (!excludeServed && recentServedIds.size) {
      let pos = 0;
      for (const id of recentServedIds) {
        servedOrder.set(String(id), pos++);
      }
    }

    const ranked = [];
    for (const entry of candidateMap.values()) {
      const track = entry?.track;
      if (!track || track.id == null) continue;
      if (isAiSlopTrack(track)) continue;

      const sid = String(track.id);
      if (seenIds.has(sid) || recentPlayedIds.has(sid)) continue;
      if (excludeServed && recentServedIds.has(sid)) continue;
      if ((target === 'new' || !allowLiked) && profile.likedIds.has(sid)) continue;
      if (recentSongKeys && hasMatchingSongKey(track, recentSongKeys)) continue;

      const fit = waveMoodFit(track, target);
      if (fit < 0) continue;
      if (strictMood && targeted && fit <= 0) continue;

      const relatedCount = entry.relatedCount || 0;
      const bestRank = entry.bestRank ?? 25;
      const seedMoodHits = entry.seedMoodHits || 0;
      const artistKey = waveArtistKey(track);
      const artistEntry = artistKey ? profile.artistWeights.get(artistKey) : null;
      const knownLikedArtist = isKnownLikedArtist(profile, track);

      let score = 0;
      // 1. Collaborative similarity evidence from SoundCloud /related (capped so cluster hubs don't monopolize every wave)
      if (relatedCount > 0) {
        score += Math.max(4, 22 - bestRank * 0.6);
        if (relatedCount > 1) {
          score += Math.min(16, 8 * (relatedCount - 1));
        }
      }
      if (entry.fromHop && !knownLikedArtist) {
        score += cursor > 0 ? 9 : 4;
      }

      // 2. Prioritize NEW similar artists instead of fixating on artists already in likes
      if (!knownLikedArtist) {
        score += relatedCount > 0 ? 18 : 6;
      } else if (seedArtists.has(artistKey) || (artistEntry?.count || 0) >= 2) {
        score -= 12;
      } else {
        score -= 4;
      }

      // 3. Search-only / artist-catalog tracks rank well when relevant
      if (entry.fromSearch && relatedCount === 0) {
        score -= 4;
      }

      // 4. User style alignment (genres, tags, title tokens, script)
      const genre = waveGenreKey(track);
      if (genre && profile.genreWeights.has(genre)) {
        score += 22 * profile.genreWeights.get(genre);
      }

      let tagMatch = 0;
      for (const tag of parseTrackTags(track.tag_list)) {
        tagMatch += profile.tagWeights.get(tag) || 0;
      }
      score += Math.min(18, tagMatch * 12);

      let tokenMatch = 0;
      for (const token of extractTitleTokens(track)) {
        tokenMatch += profile.titleTokenWeights.get(token) || 0;
      }
      score += Math.min(10, tokenMatch * 3);

      const script = detectScriptBucket(track);
      if (script && profile.scriptShare.has(script)) {
        score += 4 * profile.scriptShare.get(script);
      }

      // 5. Section / Mood alignment
      if (targeted) {
        if (fit >= 2) score += 36;
        else if (fit >= 1) score += 22;
        else if (fit > 0) score += 10;
        else if (seedMoodHits > 0) score += 6;
        else score -= 12;
      }

      // 6. Freshness boost (especially for "Новое")
      const freshness = trackFreshnessScore(track);
      if (target === 'new') {
        score += freshness * 32;
      } else {
        score += freshness * 4;
      }

      // 7. Gentle engagement tiebreaker (helps genuine tracks beat zero-play spam)
      const plays = Number(track.playback_count || 0);
      const likes = Number(track.likes_count || 0);
      if (plays > 0 || likes > 0) {
        score += Math.min(4, Math.log10(1 + plays + likes * 10) * 0.7);
      }

      // 8. Penalize tracks already shown in earlier feeds if relaxing served exclusion
      if (!excludeServed && recentServedIds.has(sid)) {
        const idx = servedOrder.get(sid) || 0;
        score -= 50 + (idx / Math.max(1, recentServedIds.size)) * 20;
      }

      // 9. Meaningful deterministic variation by cursor + salt so refreshes & sessions explore varied candidates
      score += ((hashString(`${sid}:${cursor}:${salt}`) % 29) - 14) * 0.9;

      ranked.push({ track, score, fit });
    }

    ranked.sort((a, b) => b.score - a.score);

    // Deduplicate same-song reuploads/edits (slowed/sped up/loop), keeping the highest-scoring copy
    const deduped = [];
    const seenSongs = new Set();
    for (const item of ranked) {
      if (hasMatchingSongKey(item.track, seenSongs)) continue;
      addTrackSongKeys(item.track, seenSongs);
      deduped.push(item.track);
    }
    return deduped;
  }

  function orderWaveRecommendations(tracks, limit, precedingTrack, mixMoods = false, maxPerArtist = 2, options = {}) {
    const artistCounts = new Map();
    let previousArtists = new Set(waveEffectiveArtists(precedingTrack));
    for (const art of previousArtists) {
      artistCounts.set(art, 1);
    }
    let previousMood = mixMoods ? waveTrackMood(precedingTrack) : '';

    const cursor = options.cursor || 0;
    const salt = options.salt || 0;
    const diversifyWindow = Math.max(1, options.diversifyWindow || 1);
    const seenSongs = new Set(options.recentSongKeys || []);

    const remaining = [];
    for (const track of (Array.isArray(tracks) ? tracks : [])) {
      if (!track) continue;
      if (hasMatchingSongKey(track, seenSongs)) continue;
      addTrackSongKeys(track, seenSongs);
      remaining.push({
        track,
        rank: remaining.length,
        artists: waveEffectiveArtists(track),
        mood: mixMoods ? waveTrackMood(track) : ''
      });
    }

    const ordered = [];
    while (remaining.length && ordered.length < limit) {
      const underCap = remaining.filter(({ artists }) =>
        !artists.length || artists.every(art => (artistCounts.get(art) || 0) < maxPerArtist));
      const pool = underCap.length ? underCap : remaining;

      const hasAlternative = pool.some(({ artists }) =>
        artists.length && artists.every(art => !previousArtists.has(art)));
      const hasNewArtist = ordered.length < 5 && pool.some(({ artists }) =>
        artists.length && artists.every(art => !artistCounts.has(art)));

      const eligible = [];
      for (const item of pool) {
        const { rank, artists, mood } = item;
        if (hasNewArtist && (!artists.length || artists.some(art => artistCounts.has(art)))) continue;
        if (hasAlternative && artists.some(art => previousArtists.has(art))) continue;
        const maxCount = artists.reduce((m, art) => Math.max(m, artistCounts.get(art) || 0), 0);
        const repeatMood = mood && mood === previousMood ? 6 : 0;
        const score = rank + 6 * maxCount + repeatMood;
        eligible.push({ item, score });
      }

      eligible.sort((a, b) => a.score - b.score);
      const pickPool = eligible.slice(0, Math.min(diversifyWindow, eligible.length));
      const pickedIdx = pickPool.length > 1
        ? hashString(`${pickPool[0].item.track.id}:${cursor}:${salt}:${ordered.length}`) % pickPool.length
        : 0;
      const bestPoolItem = (pickPool[pickedIdx] || eligible[0] || { item: pool[0] }).item;

      const removeIdx = remaining.indexOf(bestPoolItem);
      const [{ track, artists, mood }] = remaining.splice(removeIdx >= 0 ? removeIdx : 0, 1);
      ordered.push(track);
      for (const art of artists) {
        artistCounts.set(art, (artistCounts.get(art) || 0) + 1);
      }
      previousArtists = new Set(artists);
      previousMood = mood;
    }

    return ordered;
  }

  function selectFavoriteWaveTracks(profile, limit, options = {}) {
    const seenIds = options.seenIds || new Set();
    const excludeIds = options.excludeIds || new Set();
    const recentPlayedIds = options.recentPlayedIds || new Set();
    const recentServedIds = options.recentServedIds || new Set();
    const recentSongKeys = options.recentSongKeys || null;
    const cursor = options.cursor || 0;
    const salt = options.salt || 0;
    const precedingTrack = options.precedingTrack || null;
    const isReplenish = Boolean(options.isReplenish);
    const allowCycle = options.allowCycle !== undefined ? Boolean(options.allowCycle) : !isReplenish;

    const rawLiked = Array.isArray(profile?.likedTracks) ? profile.likedTracks : [];
    const fullLiked = rawLiked.filter(t => t && t.id != null && !isAiSlopTrack(t));
    const liked = fullLiked.length ? fullLiked : rawLiked.filter(t => t && t.id != null && !isAiSlopTrack(t, { allowShort: true }));
    if (!liked.length) return [];

    let candidates = liked.filter(t => {
      const sid = String(t.id);
      return !seenIds.has(sid) && !excludeIds.has(sid) && !recentPlayedIds.has(sid) && !recentServedIds.has(sid);
    });
    if (candidates.length < Math.min(limit, liked.length)) {
      const freshUnplayed = liked.filter(t => {
        const sid = String(t.id);
        return !seenIds.has(sid) && !excludeIds.has(sid) && !recentPlayedIds.has(sid);
      });
      if (freshUnplayed.length >= Math.min(limit, liked.length)) {
        candidates = freshUnplayed;
      } else {
        const freshNoExclude = liked.filter(t => !seenIds.has(String(t.id)) && !excludeIds.has(String(t.id)));
        if (freshNoExclude.length >= Math.min(limit, liked.length)) {
          candidates = freshNoExclude;
        } else {
          const notInQueue = liked.filter(t => !seenIds.has(String(t.id)));
          if (notInQueue.length) {
            candidates = notInQueue;
          } else if (allowCycle) {
            candidates = liked.filter(t => !excludeIds.has(String(t.id)));
            if (!candidates.length) candidates = liked;
          } else {
            return [];
          }
        }
      }
    }

    const total = Math.max(1, candidates.length - 1);
    const scored = candidates.map((track, idx) => {
      const sid = String(track.id);
      const recency = 1.0 - 0.35 * (idx / total);
      const servedPenalty = recentServedIds.has(sid) ? -14 : 0;
      const playedPenalty = recentPlayedIds.has(sid) ? -20 : 0;
      const jitter = ((hashString(`${sid}:${cursor}:${salt}`) % 37) - 18) * 0.9;
      return { track, score: recency * 12 + servedPenalty + playedPenalty + jitter };
    });

    scored.sort((a, b) => b.score - a.score);
    return orderWaveRecommendations(scored.map(s => s.track), limit, precedingTrack, true, 1, {
      cursor,
      salt,
      diversifyWindow: 3,
      recentSongKeys
    });
  }

  function filterDiverseDiscoveries(discoveries, profile, limit) {
    const freshArtists = [];
    const knownArtists = [];
    for (const track of discoveries) {
      if (isKnownLikedArtist(profile, track)) {
        knownArtists.push(track);
      } else {
        freshArtists.push(track);
      }
    }
    const maxKnown = freshArtists.length >= Math.min(limit, 8)
      ? Math.max(1, Math.floor(limit / 8))
      : limit;
    return [...freshArtists, ...knownArtists.slice(0, maxKnown)];
  }

  function blendDiscoveryWithFavorites(discoveries, profile, mood, limit, options = {}) {
    const target = normalizeWaveMood(mood);
    const seenIds = options.seenIds || new Set();
    const recentPlayedIds = options.recentPlayedIds || new Set();
    const recentServedIds = options.recentServedIds || new Set();
    const recentSongKeys = options.recentSongKeys || null;
    const precedingTrack = options.precedingTrack || null;
    const cursor = options.cursor || 0;
    const salt = options.salt || 0;

    const cleanDiscoveries = filterDiverseDiscoveries(
      (Array.isArray(discoveries) ? discoveries : []).filter(Boolean),
      profile,
      limit
    );

    // "Новое" strictly forbids already-liked tracks
    if (target === 'new') {
      return orderWaveRecommendations(cleanDiscoveries, limit, precedingTrack, false, 1, {
        cursor,
        salt,
        diversifyWindow: 3,
        recentSongKeys
      });
    }

    const rawFavPool = (profile?.likedTracks || []);
    const fullFavPool = rawFavPool.filter(t => t && t.id != null && !isAiSlopTrack(t));
    const usableFavPool = fullFavPool.length ? fullFavPool : rawFavPool.filter(t => t && t.id != null && !isAiSlopTrack(t, { allowShort: true }));

    let favorites = usableFavPool
      .filter(t => {
        const sid = String(t.id);
        if (seenIds.has(sid) || recentPlayedIds.has(sid) || recentServedIds.has(sid)) return false;
        if (recentSongKeys && hasMatchingSongKey(t, recentSongKeys)) return false;
        if (cleanDiscoveries.some(d => String(d.id) === sid)) return false;
        return waveMoodFit(t, target) >= 0;
      })
      .sort((a, b) => {
        const diff = waveMoodFit(b, target) - waveMoodFit(a, target);
        if (diff !== 0) return diff;
        return (hashString(`${a.id}:${cursor}:${salt}`) % 23) - (hashString(`${b.id}:${cursor}:${salt}`) % 23);
      });

    // When external discoveries are unavailable (e.g. offline), widen to profile pool tracks matching the mood
    if (!cleanDiscoveries.length && favorites.length < limit && Array.isArray(profile?.pool)) {
      const used = new Set(favorites.map(t => String(t.id)));
      const poolExtras = profile.pool
        .filter(t => {
          if (!t || t.id == null || isAiSlopTrack(t)) return false;
          const sid = String(t.id);
          return !seenIds.has(sid) && !recentPlayedIds.has(sid) && !used.has(sid) && waveMoodFit(t, target) >= 0;
        })
        .sort((a, b) => {
          const fitDiff = waveMoodFit(b, target) - waveMoodFit(a, target);
          if (fitDiff !== 0) return fitDiff;
          return (hashString(`${a.id}:${cursor}:${salt}`) % 23) - (hashString(`${b.id}:${cursor}:${salt}`) % 23);
        });
      favorites = [...favorites, ...poolExtras];
    }

    // Keep wave overwhelmingly fresh (~95% new discoveries); mix at most 1 familiar favorite track mid-feed in "all" (or fallback if scarce)
    const maxFavs = cleanDiscoveries.length >= 10
      ? (target === 'all' ? 1 : 0)
      : Math.max(0, limit - cleanDiscoveries.length);

    const chosenFavs = favorites.slice(0, maxFavs);
    const orderedDiscoveries = orderWaveRecommendations(
      cleanDiscoveries,
      Math.max(0, limit - chosenFavs.length),
      precedingTrack,
      target === 'all',
      1,
      { cursor, salt, diversifyWindow: 3, recentSongKeys }
    );

    if (!chosenFavs.length) return orderedDiscoveries;

    const combined = [...orderedDiscoveries];
    chosenFavs.forEach((fav, idx) => {
      const insertAt = Math.min(combined.length, 6 + idx * 7);
      combined.splice(insertAt, 0, fav);
    });
    return combined.slice(0, limit);
  }

  async function invokeRelatedTracks(api, trackId) {
    if (!api || trackId == null) return [];
    try {
      const res = typeof api.getRelatedTracks === 'function'
        ? await api.getRelatedTracks(trackId)
        : (typeof api.invoke === 'function' ? await api.invoke('get_related_tracks', { trackId }) : []);
      return Array.isArray(res) ? res : [];
    } catch (_) {
      return [];
    }
  }

  async function invokeArtistTracks(api, userId, limit = 30) {
    if (!api || userId == null) return [];
    try {
      const res = typeof api.getArtistTracks === 'function'
        ? await api.getArtistTracks(userId, limit)
        : (typeof api.invoke === 'function' ? await api.invoke('get_artist_tracks', { userId, limit }) : []);
      return Array.isArray(res) ? res : [];
    } catch (_) {
      return [];
    }
  }

  async function invokeSoundCloudSearch(api, query, offset = 0) {
    if (!api || !query) return [];
    try {
      const res = typeof api.searchTracks === 'function'
        ? await api.searchTracks(query, 'soundcloud', offset)
        : (typeof api.invoke === 'function'
          ? await api.invoke('search_tracks', offset ? { query, provider: 'soundcloud', offset } : { query, provider: 'soundcloud' })
          : []);
      return Array.isArray(res) ? res : [];
    } catch (_) {
      return [];
    }
  }

  function trimOrderedSet(set, maxSize) {
    while (set.size > maxSize) {
      const oldest = set.values().next().value;
      set.delete(oldest);
    }
  }

  function pushOrderedSet(set, value, maxSize) {
    if (value == null) return;
    const key = String(value);
    if (!key) return;
    set.delete(key);
    set.add(key);
    trimOrderedSet(set, maxSize);
  }

  function createWaveCache(ttlMs = 15 * 60 * 1000) {
    const related = new Map();
    const artistTracks = new Map();
    const searchOffsets = new Map();
    const recentSeedIds = new Set();
    const recentSeedArtists = new Set();
    const recentHopIds = new Set();
    const maxSeedHistory = 80;
    const maxArtistHistory = 40;
    const maxHopHistory = 120;

    const getCached = async (store, key, fetcher) => {
      const k = String(key);
      const hit = store.get(k);
      if (hit && Date.now() - hit.at < ttlMs) {
        return hit.data;
      }
      const data = await fetcher();
      if (Array.isArray(data) && data.length > 0) {
        store.set(k, { data, at: Date.now() });
        if (store.size > 160) {
          const oldestKey = store.keys().next().value;
          store.delete(oldestKey);
        }
      }
      return Array.isArray(data) ? data : [];
    };

    return {
      recentSeedIds,
      recentSeedArtists,
      recentHopIds,
      hydrate(state = {}) {
        for (const id of (Array.isArray(state.seedIds) ? state.seedIds : [])) {
          pushOrderedSet(recentSeedIds, id, maxSeedHistory);
        }
        for (const art of (Array.isArray(state.seedArtists) ? state.seedArtists : [])) {
          pushOrderedSet(recentSeedArtists, art, maxArtistHistory);
        }
        for (const id of (Array.isArray(state.hopIds) ? state.hopIds : [])) {
          pushOrderedSet(recentHopIds, id, maxHopHistory);
        }
      },
      exportState() {
        return {
          seedIds: Array.from(recentSeedIds),
          seedArtists: Array.from(recentSeedArtists),
          hopIds: Array.from(recentHopIds)
        };
      },
      rememberSeeds(seeds) {
        for (const s of seeds || []) {
          if (!s || s.id == null) continue;
          pushOrderedSet(recentSeedIds, s.id, maxSeedHistory);
          const art = waveArtistKey(s);
          if (art) pushOrderedSet(recentSeedArtists, art, maxArtistHistory);
        }
      },
      rememberHops(hops) {
        for (const h of hops || []) {
          if (!h || h.id == null) continue;
          pushOrderedSet(recentHopIds, h.id, maxHopHistory);
        }
      },
      nextSearchOffset(query, stride = 20, maxOffset = 80) {
        const key = String(query || '').trim().toLowerCase();
        if (!key) return 0;
        const current = searchOffsets.get(key) || 0;
        searchOffsets.set(key, (current + stride) > maxOffset ? 0 : current + stride);
        return current;
      },
      getRelated(trackId, api) {
        return getCached(related, trackId, () => invokeRelatedTracks(api, trackId));
      },
      getArtistTracks(userId, api, limit = 30) {
        return getCached(artistTracks, userId, () => invokeArtistTracks(api, userId, limit));
      },
      clear() {
        related.clear();
        artistTracks.clear();
        searchOffsets.clear();
        recentSeedIds.clear();
        recentSeedArtists.clear();
        recentHopIds.clear();
      }
    };
  }

  function recordCandidate(candidateMap, track, meta) {
    if (!track || track.id == null || isAiSlopTrack(track)) return;
    const sid = String(track.id);
    let entry = candidateMap.get(sid);
    if (!entry) {
      entry = {
        track,
        sources: new Set(),
        relatedCount: 0,
        bestRank: 99,
        seedMoodHits: 0,
        fromHop: false,
        fromArtistCatalog: false,
        fromSearch: false
      };
      candidateMap.set(sid, entry);
    }
    if (meta.sourceKey && !entry.sources.has(meta.sourceKey)) {
      entry.sources.add(meta.sourceKey);
      if (meta.isRelated) {
        entry.relatedCount += 1;
        if (meta.seedFit > 0) entry.seedMoodHits += 1;
      }
    }
    if (meta.isRelated && typeof meta.rank === 'number' && meta.rank < entry.bestRank) {
      entry.bestRank = meta.rank;
    }
    if (meta.fromHop) entry.fromHop = true;
    if (meta.fromArtistCatalog) entry.fromArtistCatalog = true;
    if (meta.fromSearch) entry.fromSearch = true;
  }

  function raceWithTimeout(tasks, timeoutMs) {
    if (!tasks.length) return Promise.resolve();
    let timer;
    return Promise.race([
      Promise.allSettled(tasks),
      new Promise(resolve => { timer = setTimeout(resolve, timeoutMs); })
    ]).finally(() => clearTimeout(timer));
  }

  async function collectWaveCandidates({
    profile,
    mood,
    cursor = 0,
    salt = 0,
    api,
    cache,
    seenIds = new Set(),
    recentPlayedIds = new Set(),
    recentServedIds = new Set(),
    recentSongKeys = null,
    queueTracks = [],
    isReplenish = false,
    maxQueueSeeds,
    timeoutMs = 3200
  }) {
    const target = normalizeWaveMood(mood);
    const targeted = target === 'energetic' || target === 'calm' || target === 'sad';
    const waveCache = cache || createWaveCache();
    const seeds = selectTasteSeeds(profile, target, {
      cursor,
      salt,
      count: 5,
      recentPlayedIds,
      recentSeedIds: waveCache.recentSeedIds,
      recentSeedArtists: waveCache.recentSeedArtists,
      queueTracks,
      maxQueueSeeds: maxQueueSeeds ?? (isReplenish ? 2 : 1)
    });
    waveCache.rememberSeeds(seeds);

    const candidateMap = new Map();
    const seedIds = new Set(seeds.map(s => String(s.id)));
    const seedArtists = new Set(seeds.map(waveArtistKey).filter(Boolean));
    const likedSongKeys = collectSongKeys(profile?.likedTracks);
    const blockedSongKeys = new Set(recentSongKeys || []);
    if (target !== 'favorites') {
      for (const k of likedSongKeys) blockedSongKeys.add(k);
    }

    // 1. Primary wave discovery: parallel SoundCloud /related across diverse taste seeds + rotated style search
    const primaryQueryLimit = (target === 'new' || targeted) ? 2 : (cursor > 0 || isReplenish ? 1 : 0);
    const primaryQueries = buildTasteSearchQueries(profile, target, cursor).slice(0, primaryQueryLimit);

    const primaryTasks = [
      ...seeds.map(seed => (async () => {
        const seedFit = waveMoodFit(seed, target);
        const list = await waveCache.getRelated(seed.id, api);
        list.slice(0, 50).forEach((track, rank) => {
          recordCandidate(candidateMap, track, {
            sourceKey: `rel:${seed.id}`,
            isRelated: true,
            rank,
            seedFit
          });
        });
      })()),
      ...primaryQueries.map(q => (async () => {
        const offset = waveCache.nextSearchOffset(q);
        const list = await invokeSoundCloudSearch(api, q, offset);
        list.slice(0, 30).forEach((track, rank) => {
          recordCandidate(candidateMap, track, {
            sourceKey: `q:${q}:${offset}`,
            isRelated: false,
            rank,
            fromSearch: true
          });
        });
      })())
    ];

    await raceWithTimeout(primaryTasks, timeoutMs);

    const rankOpts = {
      seenIds,
      recentPlayedIds,
      recentServedIds,
      excludeServed: true,
      recentSongKeys: blockedSongKeys,
      seedArtists,
      cursor,
      salt,
      allowLiked: false,
      strictMood: targeted
    };

    let ranked = rankWaveCandidates(candidateMap, profile, target, rankOpts);

    // 2. Branch outward via 2nd-hop /related through UNFAMILIAR tracks, unfamiliar artist catalogs & paginated search
    const unfamiliarCount = ranked.filter(t => !isKnownLikedArtist(profile, t)).length;
    const moodMatchedCount = targeted ? ranked.filter(t => waveMoodFit(t, target) > 0).length : unfamiliarCount;
    if (ranked.length < (targeted ? 20 : 32) || unfamiliarCount < 24 || (targeted && moodMatchedCount < 18) || cursor > 0 || isReplenish) {
      const expansionTasks = [];
      const secondHopPool = ranked.filter(t =>
        !seedIds.has(String(t.id)) && !isKnownLikedArtist(profile, t) && !waveCache.recentHopIds.has(String(t.id)));
      const hopFallback = ranked.filter(t => !seedIds.has(String(t.id)) && !waveCache.recentHopIds.has(String(t.id)));
      const hopSource = secondHopPool.length ? secondHopPool : (hopFallback.length ? hopFallback : ranked.filter(t => !seedIds.has(String(t.id))));
      const secondHopSeeds = hopSource
        .filter(t => !targeted || waveMoodFit(t, target) > 0)
        .slice(0, (cursor > 0 || isReplenish) ? 3 : 2);

      waveCache.rememberHops(secondHopSeeds);

      for (const hop of secondHopSeeds) {
        expansionTasks.push((async () => {
          const hopFit = waveMoodFit(hop, target);
          const list = await waveCache.getRelated(hop.id, api);
          list.slice(0, 40).forEach((track, rank) => {
            recordCandidate(candidateMap, track, {
              sourceKey: `hop:${hop.id}`,
              isRelated: true,
              rank: rank + 3,
              seedFit: hopFit,
              fromHop: true
            });
          });
        })());
      }

      const artistHop = secondHopSeeds.find(t => t?.user_id != null && !isKnownLikedArtist(profile, t));
      if (artistHop && (unfamiliarCount < 24 || cursor > 0)) {
        expansionTasks.push((async () => {
          const list = await waveCache.getArtistTracks(artistHop.user_id, api, 20);
          list.slice(0, 15).forEach((track, rank) => {
            recordCandidate(candidateMap, track, {
              sourceKey: `art:${artistHop.user_id}`,
              isRelated: false,
              rank: rank + 6,
              fromArtistCatalog: true,
              fromHop: true
            });
          });
        })());
      }

      if (ranked.length < (targeted ? 18 : 20) || (targeted && moodMatchedCount < 16)) {
        const fallbackQueries = buildTasteSearchQueries(profile, target, cursor + 1)
          .filter(q => !primaryQueries.includes(q))
          .slice(0, 2);
        for (const q of fallbackQueries) {
          expansionTasks.push((async () => {
            const offset = waveCache.nextSearchOffset(q);
            const list = await invokeSoundCloudSearch(api, q, offset);
            list.slice(0, 30).forEach((track, rank) => {
              recordCandidate(candidateMap, track, {
                sourceKey: `q:${q}:${offset}`,
                isRelated: false,
                rank,
                fromSearch: true
              });
            });
          })());
        }
      }

      if (expansionTasks.length) {
        await raceWithTimeout(expansionTasks, Math.min(1800, timeoutMs));
        ranked = rankWaveCandidates(candidateMap, profile, target, rankOpts);
      }
    }

    // 3. Relax served-history exclusion if fresh non-repeated discoveries are nearly exhausted
    if (ranked.length < 12 && recentServedIds.size > 0) {
      ranked = rankWaveCandidates(candidateMap, profile, target, {
        ...rankOpts,
        excludeServed: false,
        recentSongKeys: target === 'favorites' ? null : likedSongKeys
      });
    }

    // 4. Emergency fallback for targeted moods ONLY if strict mood-matched tracks are scarce
    if (targeted && ranked.length < 6) {
      const softRanked = rankWaveCandidates(candidateMap, profile, target, {
        ...rankOpts,
        strictMood: false,
        excludeServed: false
      });
      const existing = new Set(ranked.map(t => String(t.id)));
      for (const t of softRanked) {
        if (!existing.has(String(t.id))) {
          ranked.push(t);
          existing.add(String(t.id));
        }
      }
    }

    return { seeds, ranked, candidateMap };
  }

  function buildTrackRadioQueries(baseTrack, hopSeeds = [], cursor = 0, salt = 0) {
    const queries = [];
    const seen = new Set();
    const pushQ = (q) => {
      const cleaned = String(q || '').replace(/[#"'`]+/g, ' ').replace(/\s+/g, ' ').trim();
      if (!cleaned || cleaned.length < 2) return;
      const key = cleaned.toLowerCase();
      if (seen.has(key)) return;
      seen.add(key);
      queries.push(cleaned);
    };

    const baseGenre = waveGenreKey(baseTrack);
    if (baseGenre) pushQ(baseGenre);
    for (const tag of parseTrackTags(baseTrack)) {
      pushQ(tag);
    }
    for (const hop of hopSeeds) {
      const hg = waveGenreKey(hop);
      if (hg) pushQ(hg);
      for (const tag of parseTrackTags(hop).slice(0, 2)) {
        pushQ(tag);
      }
    }
    if (!queries.length && baseTrack?.artist) {
      pushQ(baseTrack.artist);
    }
    if (queries.length <= 1) return queries;
    const shift = Math.abs((cursor >>> 0) + (salt >>> 0)) % queries.length;
    return [...queries.slice(shift), ...queries.slice(0, shift)];
  }

  async function collectTrackRadioCandidates({
    baseTrack,
    api,
    cache,
    profile,
    seenIds = new Set(),
    recentPlayedIds = new Set(),
    recentServedIds = new Set(),
    recentSongKeys = new Set(),
    queueTracks = [],
    cursor = 0,
    salt = 0,
    isReplenish = false,
    timeoutMs = 2600
  }) {
    if (!baseTrack || baseTrack.id == null) {
      return { ranked: [], candidateMap: new Map() };
    }
    const waveCache = cache || createWaveCache();
    const baseId = String(baseTrack.id);
    const effectiveProfile = profile || buildTasteProfile([], [], [baseTrack, ...queueTracks]);
    const candidateMap = new Map();
    const blockedIds = new Set(seenIds);
    blockedIds.add(baseId);
    for (const qTrack of (Array.isArray(queueTracks) ? queueTracks : [])) {
      if (qTrack?.id != null) blockedIds.add(String(qTrack.id));
    }
    const blockedSongKeys = new Set(recentSongKeys || []);
    for (const k of collectSongKeys([baseTrack, ...(Array.isArray(queueTracks) ? queueTracks : [])])) {
      blockedSongKeys.add(k);
    }

    const pickDiverseHops = (pool, count) => {
      const valid = (Array.isArray(pool) ? pool : []).filter(t =>
        t && t.id != null && String(t.id) !== baseId && !isAiSlopTrack(t) && !recentPlayedIds.has(String(t.id))
      );
      const scored = valid.map(t => {
        const sid = String(t.id);
        const hopPenalty = waveCache.recentHopIds.has(sid) ? 1000 : 0;
        const servedPenalty = recentServedIds.has(sid) ? 200 : 0;
        const noise = hashString(`${sid}:${salt}:${cursor}`) % 199;
        return { t, score: hopPenalty + servedPenalty + noise };
      });
      scored.sort((a, b) => a.score - b.score);
      const picked = [];
      const pickedArtists = new Set();
      for (const item of scored) {
        const ak = waveArtistKey(item.t);
        if (ak && pickedArtists.has(ak) && picked.length < count) continue;
        picked.push(item.t);
        if (ak) pickedArtists.add(ak);
        if (picked.length >= count) break;
      }
      if (picked.length < count) {
        for (const item of scored) {
          if (picked.some(p => String(p.id) === String(item.t.id))) continue;
          picked.push(item.t);
          if (picked.length >= count) break;
        }
      }
      return picked;
    };

    let hopSeeds = [];
    const tasks = [];

    if (!isReplenish) {
      const baseRelated = await waveCache.getRelated(baseTrack.id, api).catch(() => []);
      baseRelated.slice(0, 50).forEach((track, rank) => {
        recordCandidate(candidateMap, track, {
          sourceKey: `rel:${baseId}`,
          isRelated: true,
          rank
        });
      });

      hopSeeds = pickDiverseHops(baseRelated, 3);
      waveCache.rememberHops(hopSeeds);
      for (const hop of hopSeeds) {
        tasks.push((async () => {
          const list = await waveCache.getRelated(hop.id, api).catch(() => []);
          list.slice(0, 40).forEach((track, rank) => {
            recordCandidate(candidateMap, track, {
              sourceKey: `hop:${hop.id}`,
              isRelated: true,
              rank: rank + 2,
              fromHop: true
            });
          });
        })());
      }
    } else {
      const queuePool = (Array.isArray(queueTracks) ? queueTracks : []).filter(t => t && t.id != null);
      const queueHops = pickDiverseHops(queuePool, 2);
      const anchors = [baseTrack, ...queueHops].filter(
        (t, idx, arr) => t?.id != null && arr.findIndex(x => String(x?.id) === String(t.id)) === idx
      );
      hopSeeds = queueHops;
      waveCache.rememberHops(queueHops);
      for (const anchor of anchors) {
        tasks.push((async () => {
          const list = await waveCache.getRelated(anchor.id, api).catch(() => []);
          list.slice(0, 40).forEach((track, rank) => {
            recordCandidate(candidateMap, track, {
              sourceKey: `rel:${anchor.id}`,
              isRelated: true,
              rank,
              fromHop: String(anchor.id) !== baseId
            });
          });
        })());
      }
    }

    const baseArtist = waveArtistKey(baseTrack);
    const artistHop = hopSeeds.find(t => t?.user_id != null && waveArtistKey(t) !== baseArtist)
      || hopSeeds.find(t => t?.user_id != null);
    if (artistHop) {
      tasks.push((async () => {
        const list = await waveCache.getArtistTracks(artistHop.user_id, api, 20).catch(() => []);
        list.slice(0, 15).forEach((track, rank) => {
          recordCandidate(candidateMap, track, {
            sourceKey: `art:${artistHop.user_id}`,
            isRelated: false,
            rank: rank + 5,
            fromArtistCatalog: true,
            fromHop: true
          });
        });
      })());
    }

    const queries = buildTrackRadioQueries(baseTrack, hopSeeds, cursor, salt).slice(0, isReplenish ? 1 : 2);
    for (const q of queries) {
      tasks.push((async () => {
        const offset = waveCache.nextSearchOffset(q);
        const list = await invokeSoundCloudSearch(api, q, offset).catch(() => []);
        list.slice(0, 25).forEach((track, rank) => {
          recordCandidate(candidateMap, track, {
            sourceKey: `q:${q}:${offset}`,
            isRelated: false,
            rank: rank + 4,
            fromSearch: true
          });
        });
      })());
    }

    if (tasks.length) {
      await raceWithTimeout(tasks, timeoutMs);
    }

    const rankOpts = {
      seenIds: blockedIds,
      recentPlayedIds,
      recentServedIds,
      excludeServed: true,
      recentSongKeys: blockedSongKeys,
      seedArtists: new Set(baseArtist ? [baseArtist] : []),
      cursor,
      salt,
      allowLiked: true,
      strictMood: false
    };

    let ranked = rankWaveCandidates(candidateMap, effectiveProfile, 'all', rankOpts);
    if (ranked.length < 12 && recentServedIds.size > 0) {
      const relaxed = rankWaveCandidates(candidateMap, effectiveProfile, 'all', {
        ...rankOpts,
        excludeServed: false
      });
      const existing = new Set(ranked.map(t => String(t.id)));
      for (const t of relaxed) {
        if (!existing.has(String(t.id))) {
          ranked.push(t);
          existing.add(String(t.id));
        }
      }
    }

    return { ranked, candidateMap };
  }

  function pickHomeTracks(candidates, count = 5, options = {}) {
    const excludeIds = options.excludeIds instanceof Set
      ? options.excludeIds
      : new Set((Array.isArray(options.excludeIds) ? options.excludeIds : []).map(String));
    const recentList = Array.isArray(options.recentIds)
      ? options.recentIds.map(String)
      : Array.from(options.recentIds || []).map(String);
    const recentPos = new Map();
    recentList.forEach((id, idx) => recentPos.set(id, idx));

    const unique = [];
    const seen = new Set();
    for (const t of (Array.isArray(candidates) ? candidates : [])) {
      if (!t || t.id == null || isAiSlopTrack(t)) continue;
      const id = String(t.id);
      if (seen.has(id)) continue;
      seen.add(id);
      unique.push(t);
    }

    const shuffle = (arr) => {
      const copy = [...arr];
      for (let i = copy.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [copy[i], copy[j]] = [copy[j], copy[i]];
      }
      return copy;
    };

    const fresh = shuffle(unique.filter(t => !excludeIds.has(String(t.id)) && !recentPos.has(String(t.id))));
    const older = unique
      .filter(t => !excludeIds.has(String(t.id)) && recentPos.has(String(t.id)))
      .sort((a, b) => recentPos.get(String(a.id)) - recentPos.get(String(b.id)));
    const fallback = shuffle(unique.filter(t => excludeIds.has(String(t.id))));

    const orderedPool = [...fresh, ...older, ...fallback];
    const picked = [];
    const pickedIds = new Set();
    const pickedArtists = new Set();
    const pickedSongKeys = collectSongKeys(options.excludeTracks);

    for (const t of orderedPool) {
      const id = String(t.id);
      const ak = waveArtistKey(t);
      if (pickedIds.has(id)) continue;
      if (ak && pickedArtists.has(ak)) continue;
      if (hasMatchingSongKey(t, pickedSongKeys)) continue;
      picked.push(t);
      pickedIds.add(id);
      if (ak) pickedArtists.add(ak);
      for (const k of waveSongKeys(t)) pickedSongKeys.add(k);
      if (picked.length >= count) return picked;
    }

    for (const t of orderedPool) {
      const id = String(t.id);
      if (pickedIds.has(id)) continue;
      if (hasMatchingSongKey(t, pickedSongKeys)) continue;
      picked.push(t);
      pickedIds.add(id);
      for (const k of waveSongKeys(t)) pickedSongKeys.add(k);
      if (picked.length >= count) return picked;
    }

    for (const t of orderedPool) {
      const id = String(t.id);
      if (pickedIds.has(id)) continue;
      picked.push(t);
      pickedIds.add(id);
      if (picked.length >= count) break;
    }

    return picked;
  }

  return {
    WAVE_MOODS,
    normalizeWaveMood,
    waveMetadataText,
    waveArtistKey,
    waveEffectiveArtists,
    waveSongKey,
    waveSongKeys,
    collectSongKeys,
    parseTrackTags,
    waveGenreKey,
    isAiSlopTrack,
    analyzeTrackMood,
    waveTrackMood,
    waveMoodFit,
    buildTasteProfile,
    selectTasteSeeds,
    buildTasteSearchQueries,
    trackFreshnessScore,
    rankWaveCandidates,
    orderWaveRecommendations,
    selectFavoriteWaveTracks,
    blendDiscoveryWithFavorites,
    createWaveCache,
    collectWaveCandidates,
    collectTrackRadioCandidates,
    pickHomeTracks
  };
});
