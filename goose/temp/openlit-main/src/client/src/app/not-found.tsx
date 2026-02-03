import Link from 'next/link'
import Confetti from '@/components/common/confetti'
import AuthDetailsCarousel from '@/components/(auth)/auth-details-carousel'
import { Button } from '@/components/ui/button'
import { GithubIcon, MoveRightIcon } from 'lucide-react'

export default function NotFound() {
  return (
    <div className="min-h-screen grid lg:grid-cols-2 bg-white dark:bg-white">
      <AuthDetailsCarousel />
      <div className="flex flex-col justify-center p-8 lg:p-16 bg-stone-50 relative">
        <Confetti />
        <div className="flex flex-col w-full max-w-sm mx-auto gap-12 z-10">
          <div className="text-center">
            <h1 className="text-4xl font-bold tracking-tight text-primary">
              404 – Bug or Feature?
            </h1>
            <p className="text-stone-600">
              This page drifted outside our observability zone. Let&apos;s head back to reality.
            </p>
          </div>
          <div className="flex flex-col items-center justify-center space-y-4 text-center">
            <Link href="/" className="text-primary font-bold">
              Return to Dashboard
            </Link>
          </div>
          <div className="grid grid-cols-2 text-center text-sm">
            <Link
              href={"https://github.com/openlit/openlit"}
              target="_blank"
              className="w-full"
            >
              <Button
                className={`w-full rounded-full gap-2 font-bold bg-stone-900 text-stone-50 hover:bg-stone-900/90 dark:bg-stone-900 dark:text-stone-50 dark:hover:bg-stone-900/90`}
              >
                Github
                <GithubIcon className="ml-2 w-4" />
              </Button>
            </Link>
            <Link
              href="https://docs.openlit.io/latest/overview"
              target="_blank"
            >
              <Button variant={"ghost"} className="hover:bg-stone-100 hover:text-stone-900 dark:hover:bg-stone-100 dark:hover:text-stone-900">
                <b>Documentation</b>
                <MoveRightIcon className="ml-2 h-5 w-5" />
              </Button>
            </Link>
          </div>
        </div>
      </div>
    </div>
  )
}