import { motion, AnimatePresence } from 'framer-motion';

export default function SuspenseLoader() {
  return (
    <AnimatePresence>
      <motion.div
        className="flex flex-col items-start justify-end w-screen h-screen overflow-hidden p-6"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -20 }}
        transition={{ duration: 0.3, ease: 'easeOut' }}
      >
        {/* <div className="flex gap-2 items-center justify-end">
          <GooseLogo size="small" />
          <span className="text-text-muted">Loading...</span>
        </div> */}
      </motion.div>
    </AnimatePresence>
  );
}
