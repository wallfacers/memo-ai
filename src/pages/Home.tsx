import React from "react";
import { Mic } from "lucide-react";

export function Home() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 text-center px-8">
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-primary/10">
        <Mic className="h-8 w-8 text-primary" />
      </div>
      <div>
        <h2 className="text-xl font-semibold text-foreground">开始一次会议</h2>
        <p className="mt-1.5 text-sm text-muted-foreground max-w-xs">
          在左侧输入会议标题并按 Enter，或点击 + 按钮创建新会议
        </p>
      </div>
    </div>
  );
}
