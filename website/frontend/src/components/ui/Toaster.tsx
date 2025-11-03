"use client"

import { useToast } from './use-toast';
import { CheckCircle, XCircle, Info, AlertTriangle, X } from 'lucide-react';

export function Toaster() {
  const { toasts, dismissToast } = useToast();
  
  return (
    <div className="fixed top-4 right-4 z-[9999] flex flex-col gap-2 max-w-md pointer-events-none">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`
            pointer-events-auto
            rounded-lg shadow-2xl p-4 pr-12
            border backdrop-blur-sm
            animate-in slide-in-from-right duration-300
            ${
              toast.type === 'success'
                ? 'bg-green-900/90 border-green-500/50 text-green-100'
                : toast.type === 'error'
                ? 'bg-red-900/90 border-red-500/50 text-red-100'
                : toast.type === 'warning'
                ? 'bg-yellow-900/90 border-yellow-500/50 text-yellow-100'
                : 'bg-blue-900/90 border-blue-500/50 text-blue-100'
            }
          `}
        >
          <button
            onClick={() => dismissToast(toast.id)}
            className="absolute top-2 right-2 p-1 rounded hover:bg-white/20 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
          
          <div className="flex items-start gap-3">
            <div className="flex-shrink-0 mt-0.5">
              {toast.type === 'success' && <CheckCircle className="w-5 h-5" />}
              {toast.type === 'error' && <XCircle className="w-5 h-5" />}
              {toast.type === 'warning' && <AlertTriangle className="w-5 h-5" />}
              {toast.type === 'info' && <Info className="w-5 h-5" />}
            </div>
            
            <div className="flex-1 min-w-0">
              <h4 className="font-bold text-sm mb-1">{toast.title}</h4>
              {toast.message && (
                <p className="text-sm opacity-90 break-words">{toast.message}</p>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

