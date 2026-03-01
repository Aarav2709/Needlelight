/**
 * Apply game-based accent color theming.
 * Hollow Knight: deep purple (#330055)
 * Silksong: red (#e92337)
 */
export function applyGameTheme(game: string) {
  const html = document.documentElement
  html.classList.remove('game-hollow-knight', 'game-silksong')

  if (game === 'silksong') {
    html.classList.add('game-silksong')
  } else {
    html.classList.add('game-hollow-knight')
  }
}
