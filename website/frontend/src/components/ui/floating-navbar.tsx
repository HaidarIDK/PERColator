"use client"

import { motion } from "motion/react"
import { useState, useEffect } from "react"
import { cn } from "@/lib/utils"
import { Menu, X, Home, Info, Settings, User } from "lucide-react"
import { usePathname } from "next/navigation"

interface NavItem {
  name: string
  href: string
  icon: React.ReactNode
}

export function FloatingNavbar() {
  const [isScrolled, setIsScrolled] = useState(false)
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false)
  const pathname = usePathname()

  // Different nav items based on current page
  const navItems: NavItem[] = pathname === '/info' 
    ? [
        { name: "Home", href: "/", icon: <Home className="w-5 h-5" /> },
        { name: "Architecture", href: "/#architecture", icon: <Settings className="w-5 h-5" /> },
        { name: "Updates", href: "/updates", icon: <User className="w-5 h-5" /> },
      ]
    : pathname === '/updates'
    ? [
        { name: "Home", href: "/", icon: <Home className="w-5 h-5" /> },
        { name: "Architecture", href: "/#architecture", icon: <Settings className="w-5 h-5" /> },
        { name: "Read More", href: "/info", icon: <Info className="w-5 h-5" /> },
      ]
    : [
        { name: "Home", href: "#home", icon: <Home className="w-5 h-5" /> },
        { name: "Read More", href: "/info", icon: <Info className="w-5 h-5" /> },
        { name: "Architecture", href: "#architecture", icon: <Settings className="w-5 h-5" /> },
        { name: "Updates", href: "/updates", icon: <User className="w-5 h-5" /> },
      ]

  useEffect(() => {
    const handleScroll = () => {
      setIsScrolled(window.scrollY > 50)
    }

    window.addEventListener("scroll", handleScroll)
    return () => window.removeEventListener("scroll", handleScroll)
  }, [])

  const handleNavClick = (href: string) => {
    if (href.startsWith('/')) {
      // External link - navigate to page
      window.location.href = href
    } else {
      // Anchor link - scroll to section
      const element = document.querySelector(href)
      if (element) {
        element.scrollIntoView({ behavior: "smooth" })
      }
    }
    setIsMobileMenuOpen(false)
  }

  return (
    <>
      {/* Floating Navbar */}
      <motion.nav
        initial={{ y: -100, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ duration: 0.6, delay: 0.5 }}
        className={cn(
          "fixed z-50 transition-all duration-300",
          // Desktop: top center
          "md:top-6 md:left-1/2 md:transform md:-translate-x-1/2",
          // Mobile: left side
          "top-4 left-4",
          isScrolled && "md:top-4"
        )}
      >
        <div className="relative">
          {/* Main navbar container */}
          <div className="bg-black/20 backdrop-blur-md border-[#B8B8FF]/30 px-4 py-3 shadow-[0_0_20px_rgba(184,184,255,0.3)] rounded-2xl md:rounded-full md:px-6">
            {/* Desktop Navigation - Horizontal */}
            <div className="hidden md:flex flex-row items-center space-x-1">
              {navItems.map((item, index) => (
                <motion.button
                  key={item.name}
                  initial={{ opacity: 0, y: -20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.4, delay: 0.7 + index * 0.1 }}
                  onClick={() => handleNavClick(item.href)}
                  className="group relative px-4 py-2 rounded-xl text-white hover:text-[#B8B8FF] transition-all duration-300 hover:bg-[#B8B8FF]/10"
                >
                  <div className="flex items-center space-x-2">
                    <div className="text-[#B8B8FF]/60 group-hover:text-[#B8B8FF] transition-colors duration-300">
                      {item.icon}
                    </div>
                    <span className="text-sm font-medium">{item.name}</span>
                  </div>
                  
                  {/* Hover effect */}
                  <div className="absolute inset-0 bg-gradient-to-r from-[#B8B8FF]/0 via-[#B8B8FF]/5 to-[#B8B8FF]/0 rounded-xl opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
                </motion.button>
              ))}
              
              {/* PerpDEX Button */}
              <motion.a
                initial={{ opacity: 0, scale: 0.8 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.4, delay: 1.0 }}
                href="/dashboard"
                className="ml-2 px-4 py-2 rounded-xl bg-gradient-to-r from-[#B8B8FF]/20 to-[#B8B8FF]/30 text-white font-semibold hover:from-[#B8B8FF]/30 hover:to-[#B8B8FF]/40 transition-all duration-300 shadow-[0_0_15px_rgba(184,184,255,0.3)] hover:shadow-[0_0_20px_rgba(184,184,255,0.5)] relative"
              >
                <span className="text-sm">PerpDEX →</span>
                <span className="absolute -top-1 -right-1 px-1.5 py-0.5 text-[10px] bg-green-500/90 text-white rounded-full font-bold">
                  LIVE
                </span>
              </motion.a>
            </div>

            {/* Mobile Menu Button */}
            <div className="md:hidden">
              <motion.button
                initial={{ opacity: 0, scale: 0.8 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.4, delay: 0.7 }}
                onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
                className="p-2 rounded-xl text-white hover:text-[#B8B8FF] hover:bg-[#B8B8FF]/10 transition-all duration-300"
              >
                {isMobileMenuOpen ? (
                  <X className="w-6 h-6" />
                ) : (
                  <Menu className="w-6 h-6" />
                )}
              </motion.button>
            </div>
          </div>

          {/* Mobile Menu Dropdown */}
          <motion.div
            initial={{ opacity: 0, y: -20, scale: 0.95 }}
            animate={{
              opacity: isMobileMenuOpen ? 1 : 0,
              y: isMobileMenuOpen ? 0 : -20,
              scale: isMobileMenuOpen ? 1 : 0.95,
            }}
            transition={{ duration: 0.3 }}
            className={cn(
              "absolute top-full left-0 mt-2 bg-black/30 backdrop-blur-md border border-[#B8B8FF]/30 rounded-2xl shadow-[0_0_20px_rgba(184,184,255,0.3)] overflow-hidden min-w-[200px]",
              isMobileMenuOpen ? "block" : "hidden"
            )}
          >
            <div className="py-2">
              {navItems.map((item, index) => (
                <motion.button
                  key={item.name}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: isMobileMenuOpen ? 1 : 0, x: isMobileMenuOpen ? 0 : -20 }}
                  transition={{ duration: 0.3, delay: index * 0.1 }}
                  onClick={() => handleNavClick(item.href)}
                  className="w-full px-5 py-3 text-left text-white hover:text-[#B8B8FF] hover:bg-[#B8B8FF]/10 transition-all duration-300 flex items-center space-x-3"
                >
                  <div className="text-[#B8B8FF]/60 flex-shrink-0">
                    {item.icon}
                  </div>
                  <span className="text-sm font-medium whitespace-nowrap">{item.name}</span>
                </motion.button>
              ))}
              
              {/* Divider */}
              <div className="mx-3 my-2 h-px bg-[#B8B8FF]/20"></div>
              
              {/* PerpDEX Button - Mobile */}
              <motion.a
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: isMobileMenuOpen ? 1 : 0, x: isMobileMenuOpen ? 0 : -20 }}
                transition={{ duration: 0.3, delay: navItems.length * 0.1 }}
                href="/dashboard"
                className="block mx-3 my-2 px-4 py-3 rounded-xl bg-gradient-to-r from-[#B8B8FF]/20 to-[#B8B8FF]/30 text-white font-semibold hover:from-[#B8B8FF]/30 hover:to-[#B8B8FF]/40 transition-all duration-300 shadow-[0_0_15px_rgba(184,184,255,0.3)] text-center relative"
                onClick={() => setIsMobileMenuOpen(false)}
              >
                <div className="flex items-center justify-center gap-2">
                  <span className="text-sm">PerpDEX →</span>
                  <span className="px-1.5 py-0.5 text-[10px] bg-green-500/90 text-white rounded-full font-bold">
                    LIVE
                  </span>
                </div>
              </motion.a>
            </div>
          </motion.div>
        </div>
      </motion.nav>

      {/* Backdrop blur overlay for mobile menu */}
      {isMobileMenuOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={() => setIsMobileMenuOpen(false)}
          className="fixed inset-0 bg-black/20 backdrop-blur-sm z-40 md:hidden"
        />
      )}
    </>
  )
}
