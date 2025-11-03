"use client"

import { useEffect, useState } from 'react';
import { EXPLORERS, NETWORK } from '@/lib/program-config';
import { CheckCircle, XCircle, Loader2, ExternalLink } from 'lucide-react';

export type TransactionState = 'idle' | 'building' | 'signing' | 'sending' | 'confirming' | 'success' | 'error';

interface TransactionStatusProps {
  state: TransactionState;
  signature?: string;
  error?: string;
  onClose?: () => void;
}

export function TransactionStatus({ state, signature, error, onClose }: TransactionStatusProps) {
  const [timeElapsed, setTimeElapsed] = useState(0);

  useEffect(() => {
    if (state === 'idle' || state === 'success' || state === 'error') {
      setTimeElapsed(0);
      return;
    }

    const startTime = Date.now();
    const interval = setInterval(() => {
      setTimeElapsed(Math.floor((Date.now() - startTime) / 1000));
    }, 100);

    return () => clearInterval(interval);
  }, [state]);

  if (state === 'idle') return null;

  const getStateInfo = () => {
    switch (state) {
      case 'building':
        return {
          icon: <Loader2 className="w-5 h-5 animate-spin text-blue-400" />,
          title: 'Building Transaction',
          description: 'Creating transaction instructions...',
          color: 'blue',
        };
      case 'signing':
        return {
          icon: <Loader2 className="w-5 h-5 animate-spin text-yellow-400" />,
          title: 'Waiting for Signature',
          description: 'Please approve in your wallet',
          color: 'yellow',
        };
      case 'sending':
        return {
          icon: <Loader2 className="w-5 h-5 animate-spin text-purple-400" />,
          title: 'Sending Transaction',
          description: 'Broadcasting to Solana network...',
          color: 'purple',
        };
      case 'confirming':
        return {
          icon: <Loader2 className="w-5 h-5 animate-spin text-blue-400" />,
          title: 'Confirming',
          description: 'Waiting for network confirmation...',
          color: 'blue',
        };
      case 'success':
        return {
          icon: <CheckCircle className="w-5 h-5 text-green-400" />,
          title: 'Transaction Confirmed!',
          description: 'Your transaction was successful',
          color: 'green',
        };
      case 'error':
        return {
          icon: <XCircle className="w-5 h-5 text-red-400" />,
          title: 'Transaction Failed',
          description: error || 'An error occurred',
          color: 'red',
        };
      default:
        return null;
    }
  };

  const info = getStateInfo();
  if (!info) return null;

  return (
    <div
      className={`fixed top-4 right-4 z-50 w-96 bg-zinc-900 border border-${info.color}-500/50 rounded-xl shadow-2xl p-4 backdrop-blur-sm animate-in slide-in-from-right`}
    >
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 mt-0.5">{info.icon}</div>
        <div className="flex-1 min-w-0">
          <h3 className={`text-sm font-semibold text-${info.color}-400 mb-1`}>
            {info.title}
          </h3>
          <p className="text-xs text-zinc-400 mb-2">{info.description}</p>

          {(state === 'building' || state === 'signing' || state === 'sending' || state === 'confirming') && (
            <div className="text-xs text-zinc-500">
              Time elapsed: {timeElapsed}s
            </div>
          )}

          {signature && (
            <a
              href={EXPLORERS.transaction(signature, NETWORK.cluster)}
              target="_blank"
              rel="noopener noreferrer"
              className={`inline-flex items-center gap-1 text-xs text-${info.color}-400 hover:text-${info.color}-300 mt-2`}
            >
              View on Explorer
              <ExternalLink className="w-3 h-3" />
            </a>
          )}

          {(state === 'success' || state === 'error') && onClose && (
            <button
              onClick={onClose}
              className="mt-3 w-full py-2 px-3 text-xs bg-zinc-800 hover:bg-zinc-700 rounded-lg transition-colors"
            >
              Close
            </button>
          )}
        </div>

        {state !== 'success' && state !== 'error' && (
          <div className="flex-shrink-0">
            <button
              onClick={onClose}
              className="text-zinc-500 hover:text-zinc-300 transition-colors"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

// Hook to manage transaction state
export function useTransactionStatus() {
  const [state, setState] = useState<TransactionState>('idle');
  const [signature, setSignature] = useState<string>();
  const [error, setErrorMessage] = useState<string>();

  const reset = () => {
    setState('idle');
    setSignature(undefined);
    setErrorMessage(undefined);
  };

  const setBuilding = () => setState('building');
  const setSigning = () => setState('signing');
  const setSending = () => setState('sending');
  const setConfirming = () => setState('confirming');
  const setSuccess = (sig: string) => {
    setSignature(sig);
    setState('success');
  };
  const setError = (err: string) => {
    setErrorMessage(err);
    setState('error');
  };

  return {
    state,
    signature,
    error,
    reset,
    setBuilding,
    setSigning,
    setSending,
    setConfirming,
    setSuccess,
    setError,
  };
}

