import React, {useState} from "react";
import {MaterialIcon} from "./MaterialIcon.tsx";

interface CopyCodeButtonProps {
    code: string;
}

export const CopyCodeButton: React.FC<CopyCodeButtonProps> = ({code}) => {
    const [copied, setCopied] = useState<boolean>(false);

    const copyToClipboard = async () => {
        try {
            await navigator.clipboard.writeText(code);
            setCopied(true);
            setTimeout(() => setCopied(false), 2000); // Reset copied state after 2 seconds
        } catch (err) {
            console.error('Failed to copy text to clipboard', err);
        }
    };

    return (
        <button onClick={copyToClipboard} className="copy-code-button" disabled={copied}>
            {copied ? <MaterialIcon icon="check_circle" /> : <MaterialIcon icon="content_copy" />}
        </button>
    );
};