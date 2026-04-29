// Claw Desktop - 微信登录面板 - 微信二维码扫码登录和状态轮询
import React, { useState, useEffect, useCallback, useRef } from 'react';
import { weixinApi, WeixinQRCode, WeixinQRStatus } from '../../api/weixin';

export const WeixinLoginPanel: React.FC<{ agentId?: string }> = ({ agentId }) => {
  const [qrCode, setQrCode] = useState<WeixinQRCode | null>(null);
  const [status, setStatus] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchQRCode = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const data = await weixinApi.getQRCode();
      setQrCode(data);
      setStatus('waiting');
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to get QR code');
    } finally {
      setLoading(false);
    }
  }, []);

  const startPolling = useCallback((qrcodeId: string) => {
    if (pollRef.current) clearInterval(pollRef.current);
    pollRef.current = setInterval(async () => {
      try {
        const data = await weixinApi.getQRStatus(qrcodeId);
        const s = data.status || '';
        setStatus(s);
        if (s === 'confirmed') {
          if (pollRef.current) clearInterval(pollRef.current);
        } else if (s === 'expired') {
          if (pollRef.current) clearInterval(pollRef.current);
          setStatus('expired');
        }
      } catch {
        if (pollRef.current) clearInterval(pollRef.current);
      }
    }, 3000);
  }, []);

  useEffect(() => {
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, []);

  useEffect(() => {
    if (qrCode?.qrcode_id) {
      startPolling(qrCode.qrcode_id);
    }
  }, [qrCode, startPolling]);

  const getStatusText = () => {
    switch (status) {
      case 'waiting': return 'Waiting for scan...';
      case 'scaned': return 'Scanned! Waiting for confirmation...';
      case 'confirmed': return 'Login successful!';
      case 'expired': return 'QR code expired. Click to refresh.';
      default: return '';
    }
  };

  const getStatusColor = () => {
    switch (status) {
      case 'waiting': return 'text-yellow-400';
      case 'scaned': return 'text-blue-400';
      case 'confirmed': return 'text-green-400';
      case 'expired': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  return (
    <div className="p-4 space-y-4">
      <h3 className="text-lg font-semibold text-gray-200">WeChat Login</h3>

      {error && (
        <div className="bg-red-900/30 border border-red-700/50 rounded-lg p-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      <div className="flex flex-col items-center justify-center py-6">
        {loading ? (
          <div className="text-gray-400">Loading QR code...</div>
        ) : qrCode?.qrcode ? (
          <div className="space-y-4 text-center">
            <div className="bg-white p-3 rounded-xl inline-block">
              <img src={qrCode.qrcode} alt="WeChat QR Code" className="w-48 h-48" />
            </div>
            <p className={`text-sm font-medium ${getStatusColor()}`}>
              {getStatusText()}
            </p>
          </div>
        ) : (
          <div className="text-center space-y-4">
            <div className="w-48 h-48 bg-gray-700/50 rounded-xl flex items-center justify-center">
              <svg className="w-16 h-16 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 4v1m6 11h2m-6 0h-2v4m0-11v3m0 0h.01M12 12h4.01M16 20h4M4 12h4m12 0h.01M5 8h2a1 1 0 001-1V5a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1zm12 0h2a1 1 0 001-1V5a1 1 0 00-1-1h-2a1 1 0 00-1 1v2a1 1 0 001 1zM5 20h2a1 1 0 001-1v-2a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1z" />
              </svg>
            </div>
            <p className="text-gray-500 text-sm">Click below to get QR code for WeChat login</p>
          </div>
        )}
      </div>

      <div className="flex justify-center">
        <button
          onClick={fetchQRCode}
          disabled={loading}
          className="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white rounded-lg transition-colors"
        >
          {status === 'expired' ? 'Refresh QR Code' : 'Get QR Code'}
        </button>
      </div>

      {status === 'confirmed' && (
        <div className="bg-green-900/30 border border-green-700/50 rounded-lg p-4 text-center">
          <p className="text-green-300 font-medium">WeChat connected successfully!</p>
          <p className="text-green-400/70 text-sm mt-1">You can now send and receive messages through WeChat.</p>
        </div>
      )}
    </div>
  );
};
