// Claw Desktop - 微信渠道 API
// 提供微信二维码登录的获取和状态轮询接口
import { httpGet, httpPost } from '../ws/http';

/** 微信二维码数据 */
export interface WeixinQRCode {
  qrcode?: string;             // 二维码图片 Base64（可选）
  qrcode_id?: string;          // 二维码会话 ID（可选）
}

/** 微信二维码扫码状态 */
export interface WeixinQRStatus {
  status?: string;             // 扫码状态：waiting/scanned/confirmed/expired
  ilink_bot_id?: string;       // iLink 机器人 ID
  bot_token?: string;          // 机器人令牌
  baseurl?: string;            // API 基础 URL
  ilink_user_id?: string;      // iLink 用户 ID
}

/** 微信渠道 API 集合 */
export const weixinApi = {
  /** 获取微信登录二维码 */
  getQRCode: () => httpGet<WeixinQRCode>('/api/weixin/qrcode'),
  /** 轮询二维码扫码状态 */
  getQRStatus: (qrcodeId: string) => httpPost<WeixinQRStatus>('/api/weixin/qr-status', { qrcode_id: qrcodeId }),
};
