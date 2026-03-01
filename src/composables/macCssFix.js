import cssContent from '@/assets/stylesheets/macFix.css?inline'

export async function useCheckDisableMouseover() {
  const source = `${navigator.userAgentData?.platform ?? ''} ${navigator.platform ?? ''} ${navigator.userAgent ?? ''}`.toLowerCase()
  const shouldDisableMouseover = source.includes('mac')

  if (shouldDisableMouseover) {
    const styleElement = document.createElement('style')
    styleElement.innerHTML = cssContent
    document.head.appendChild(styleElement)
  }
}
