import {useEffect, useRef} from "react";

export function useScrollToBottom(...dependencies: any[]) {
    const bottomRef = useRef<HTMLDivElement>(null)
    const bottom = <div ref={bottomRef}/>
    useEffect(() => {
        if (bottomRef.current) {
            bottomRef.current.scrollIntoView({ behavior: "smooth" });
        }
    }, dependencies);
    return bottom
}