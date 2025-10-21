"use client"

import { motion, AnimatePresence } from "framer-motion"
import { CheckCircle, XCircle, AlertCircle, Info, X } from "lucide-react"

interface ToastProps {
  message: string
  type: 'success' | 'error' | 'warning' | 'info'
  onClose: () => void
}

export function Toast({ message, type, onClose }: ToastProps) {
  const getIcon = () => {
    switch (type) {
      case 'success': return <CheckCircle className="w-5 h-5" />
      case 'error': return <XCircle className="w-5 h-5" />
      case 'warning': return <AlertCircle className="w-5 h-5" />
      case 'info': return <Info className="w-5 h-5" />
    }
  }

  const getColors = () => {
    switch (type) {
      case 'success': return 'from-green-900/90 to-emerald-900/80 border-green-500/50 text-green-300'
      case 'error': return 'from-red-900/90 to-rose-900/80 border-red-500/50 text-red-300'
      case 'warning': return 'from-amber-900/90 to-yellow-900/80 border-amber-500/50 text-amber-300'
      case 'info': return 'from-blue-900/90 to-cyan-900/80 border-blue-500/50 text-blue-300'
    }
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: -20, scale: 0.95 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: -20, scale: 0.95 }}
      className={`bg-gradient-to-r ${getColors()} border-2 rounded-lg p-4 shadow-2xl backdrop-blur-lg max-w-md`}
    >
      <div className="flex items-start gap-3">
        <div className="mt-0.5">
          {getIcon()}
        </div>
        <div className="flex-1">
          <p className="text-sm font-medium whitespace-pre-line">{message}</p>
        </div>
        <button
          onClick={onClose}
          className="text-gray-400 hover:text-white transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </motion.div>
  )
}

interface ToastContainerProps {
  toasts: Array<{ id: string; message: string; type: 'success' | 'error' | 'warning' | 'info' }>
  onClose: (id: string) => void
}

export function ToastContainer({ toasts, onClose }: ToastContainerProps) {
  return (
    <div className="fixed top-4 right-4 z-[9999] space-y-2">
      <AnimatePresence>
        {toasts.map((toast) => (
          <Toast
            key={toast.id}
            message={toast.message}
            type={toast.type}
            onClose={() => onClose(toast.id)}
          />
        ))}
      </AnimatePresence>
    </div>
  )
}

