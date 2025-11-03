"use client"

import { useState, useEffect } from 'react';

export type ToastType = 'success' | 'error' | 'info' | 'warning';

export interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;
}

const toasts: Toast[] = [];
const listeners: Set<(toasts: Toast[]) => void> = new Set();

function notifyListeners() {
  listeners.forEach((listener) => listener([...toasts]));
}

export function toast({ type, title, message, duration = 5000 }: Omit<Toast, 'id'>) {
  const id = Math.random().toString(36).substring(7);
  const newToast: Toast = { id, type, title, message, duration };
  
  toasts.push(newToast);
  notifyListeners();
  
  if (duration > 0) {
    setTimeout(() => {
      const index = toasts.findIndex((t) => t.id === id);
      if (index > -1) {
        toasts.splice(index, 1);
        notifyListeners();
      }
    }, duration);
  }
  
  return id;
}

export function dismissToast(id: string) {
  const index = toasts.findIndex((t) => t.id === id);
  if (index > -1) {
    toasts.splice(index, 1);
    notifyListeners();
  }
}

export function useToast() {
  const [currentToasts, setCurrentToasts] = useState<Toast[]>([]);
  
  useEffect(() => {
    const listener = (newToasts: Toast[]) => setCurrentToasts(newToasts);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, []);
  
  return { toasts: currentToasts, toast, dismissToast };
}

