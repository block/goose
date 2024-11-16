import React from "react";
import { Card } from "./components/ui/card"

export default function Splash() {
  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-8">
      <div className="flex items-center gap-2">
        <span className="text-xl font-medium">🪿 ask goose</span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 w-full max-w-xl">
        <Card className="p-4 cursor-pointer hover:bg-gray-50">
          <p className="text-sm">Migrate code to React</p>
        </Card>
        <Card className="p-4 cursor-pointer hover:bg-gray-50">
          <p className="text-sm">Scaffold this API for data retention</p>
        </Card>
        <Card className="p-4 cursor-pointer hover:bg-gray-50">
          <p className="text-sm">Summarize my recent file changes</p>
        </Card>
        <Card className="p-4 cursor-pointer hover:bg-gray-50">
          <p className="text-sm">Find all .pdf files</p>
        </Card>
      </div>
    </div>
  )
}
