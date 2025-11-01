'use client';

import { useEffect } from 'react';

export interface ToastProps {
  id: string;
  message: string;
  type: 'success' | 'error' | 'info' | 'warning';
  onClose: (id: string) => void;
}

export default function Toast({ id, message, type, onClose }: ToastProps) {
  useEffect(() => {
    const timer = setTimeout(() => {
      onClose(id);
    }, 5000); // Auto-close after 5 seconds

    return () => clearTimeout(timer);
  }, [id, onClose]);

  const bgColor = {
    success: 'bg-green-600',
    error: 'bg-red-600',
    info: 'bg-blue-600',
    warning: 'bg-yellow-600',
  }[type];

  const icon = {
    success: '✓',
    error: '✕',
    info: 'ℹ',
    warning: '⚠',
  }[type];

  return (
    <div
      className={`${bgColor} text-white px-4 py-3 rounded-lg shadow-lg min-w-[300px] max-w-[400px] mb-3 animate-slide-in-right`}
    >
      <div className="flex items-start gap-3">
        <div className="text-xl font-bold shrink-0">{icon}</div>
        <div className="flex-1 text-sm leading-relaxed break-words">{message}</div>
        <button
          onClick={() => onClose(id)}
          className="text-white/80 hover:text-white text-xl leading-none shrink-0 -mt-1"
        >
          ×
        </button>
      </div>
    </div>
  );
}

