export default function NotFound() {
  return (
    <div className="min-h-screen bg-black flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-white mb-4">404</h1>
        <p className="text-gray-400 mb-8">Page not found</p>
        <a href="/" className="text-[#B8B8FF] hover:text-[#B8B8FF]/80 underline">
          Return to home
        </a>
      </div>
    </div>
  );
}


