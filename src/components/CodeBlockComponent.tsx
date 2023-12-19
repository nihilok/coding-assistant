import React from "react";
import {CopyCodeButton} from "./CopyCodeButton.tsx";
import {Prism as SyntaxHighlighter} from "react-syntax-highlighter";
import {darcula} from "react-syntax-highlighter/dist/esm/styles/prism";

interface CodeComponentProps {
    className?: string;
    children: React.ReactNode;
    // Other properties from the markdown parser can be added here
}

const CodeBlockComponent: React.FC<CodeComponentProps> = ({children, className, ...rest}) => {
    const match = /language-(\w+)/.exec(className || '');

    const codeString = String(children).replace(/\n$/, '');
    const language = match ? match[1] : '';

    return match ? (
        <div className="code-container">
            {!!language && <div className="language">{language}</div>}
            <CopyCodeButton code={codeString}/>
            <SyntaxHighlighter
                {...rest}
                inline={"true"}
                PreTag="div"
                children={codeString}
                language={language}
                style={darcula}
                customStyle={{fontSize: "0.9rem"}}
            />
        </div>
    ) : (
        <div className="code-container">
            <CopyCodeButton code={codeString}/>
            <code>
                {children}
            </code>
        </div>
    );
};
export const MemoizedCodeBlock = React.memo(CodeBlockComponent)