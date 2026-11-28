function renderPlayerTitle(track) {
  const title = document.getElementById('player-title');
  const link = document.getElementById('open-sc-permalink-btn');
  title.textContent = track ? track.title || 'SoundCloud Track' : 'Трек не выбран';
  link.removeAttribute('href');
  if (!track) return;

  if (URL.canParse(track.permalink_url)) {
    const url = new URL(track.permalink_url);
    if (url.protocol === 'https:' && ['soundcloud.com', 'www.soundcloud.com', 'm.soundcloud.com', 'on.soundcloud.com'].includes(url.hostname) && !url.username && !url.password) {
      link.href = url.href;
      return;
    }
  }
  const query = [track.artist, track.title].filter(Boolean).join(' ').trim();
  if (query) link.href = 'https://soundcloud.com/search/sounds?q=' + encodeURIComponent(query);
}
