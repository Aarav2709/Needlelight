import { getVersion } from '@tauri-apps/api/app'
import { fetch } from '@tauri-apps/plugin-http'

export const useFetch = async (url, item, isSilent) => {
  try {
    const version = await getVersion().catch(() => '0.0.0')
    return await fetch(url, {
      method: 'GET',
      headers: { 'User-Agent': `needlelight/desktop/${version} (support@needlelight.app)` },
    })
  } catch (err) {
    if (!isSilent) {
      throw err
    } else {
      console.error(err)
    }
  }
}
