"use client"

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string }
  reset: () => void
}) {
  return (
    <div className="min-h-screen bg-black flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-white mb-4">Error</h1>
        <p className="text-gray-400 mb-4">Something went wrong</p>
        <p className="text-gray-500 text-sm mb-8">{error.message || "An unexpected error occurred"}</p>
        <button
          onClick={reset}
          className="px-6 py-3 bg-[#B8B8FF] text-black rounded-lg font-semibold hover:bg-[#B8B8FF]/80 transition-colors"
        >
          Try again
        </button>
      </div>
    </div>
  );
}


