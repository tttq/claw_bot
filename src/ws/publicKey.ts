/**
 * 🔐 内嵌 RSA 公钥 - 自动生成文件，请勿手动修改!
 *
 * 生成命令: node fix-pubkey.js
 * 更新时间: 2026-04-12 11:25:05
 */

const EMBEDDED_KEY_B64 = 'LS0tLS1CRUdJTiBQVUJMSUMgS0VZLS0tLS0NCk1JSUJJakFOQmdrcWhraUc5dzBCQVFFRkFBT0NBUThBTUlJQkNnS0NBUUVBeWlUZ1AzSzVwNTlJTjRWQjVURGENClU4ck9Hd2VzUS9QbitlQWxBaTl6S1VzakNvNFM2MXVlalZWSkJHcWsyYzNCY290WXY3S2FtMVp3RHNjbzRuN2cNClhlcVNxK1JlbnZXU0t3SW8zelN5MHM5NUZWa2o3R2Z5RXlVVzEyNVNRQTl3dnZQU1NQaGd3T0lXelBUTitwUXgNCnRzcnZwRWlMRE9nTGJmUFN6THVYZGFHZ0NzMDNUbjhlM1UwRkZiR3NOM0wxVjhTTWx2NXNHK2pTazlKNHl4UW8NCnUxTVJmbTFDWjZYZXBiOHRld0RZUEV2SERvc0hHV2ptQ0dDQk5md0hteFo0VkxEOVd6aWhnRC8wR2FBRVJGaHoNCk5BaU1yL1NQQzNVdG1IbTFLMHA5S2gzY2ZwV21lb0dUZndNeG05Z1NpazNPdjVhOHlqV1BPbXpzaWJnZHBRZk4NCllRSURBUUFCDQotLS0tLUVORCBQVUJMSUMgS0VZLS0tLS0='

export function getEmbeddedPublicKey(): string {
  try {
    const decoded = decodeURIComponent(escape(atob(EMBEDDED_KEY_B64)))
    if (!decoded.includes('-----BEGIN')) {
      throw new Error('[Security] Embedded public key is corrupted')
    }
    return decoded
  } catch (e) {
    throw new Error('[Security] Embedded public key is corrupted: ' + (e as Error).message)
  }
}

export function getEnvPublicKey(): string {
  const envKey = (import.meta as unknown as { env?: { VITE_PUBLIC_KEY?: string } }).env?.VITE_PUBLIC_KEY
  if (envKey?.includes('-----BEGIN')) return envKey
  return ''
}

export function getDefaultPublicKey(): string {
  return getEmbeddedPublicKey() || getEnvPublicKey()
}
