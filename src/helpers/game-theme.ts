// applies the game accent theme, deep purple for hollow knight and red for silksong
export function applyGameTheme(game: string) {
  const html = document.documentElement
  html.classList.remove('game-hollow-knight', 'game-silksong')

  if (game === 'silksong') {
    html.classList.add('game-silksong')
  } else {
    html.classList.add('game-hollow-knight')
  }

  // remembered so the next launch's loading screen already has the right color
  try {
    localStorage.setItem('needlelight.game', game)
  } catch {
    // storage unavailable, so the loading screen uses the default color
  }
}
