import React, {useEffect, useRef, useState} from "react";

export function CustomTextArea({setValue}: {
    setValue: React.Dispatch<React.SetStateAction<string>>;
}) {
    const [value, setInternalValue] = useState("")
    const textAreaRef = useRef<HTMLTextAreaElement>(null);
    const cursorPositionRef = useRef<number | null>(null);

    useEffect(() => {
        if (textAreaRef.current && cursorPositionRef.current !== null) {
            textAreaRef.current.selectionStart = cursorPositionRef.current;
            textAreaRef.current.selectionEnd = cursorPositionRef.current;
            cursorPositionRef.current = null; // Reset the cursor position after setting it
        }
    }, [value]);

    const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
        if (!["Tab", "Enter", "Home", "End", "`"].includes(event.key)) return;
        if (event.key === 'Tab') {
            event.preventDefault();
            const textArea = event.target as HTMLTextAreaElement;
            const cursorPosition = textArea.selectionStart;
            const beforeCursor = value.substring(0, cursorPosition);
            const afterCursor = value.substring(cursorPosition);

            if (event.shiftKey) {
                // For Shift+Tab, remove 4 spaces from the start of the line (if present).

                const lineStart = beforeCursor.lastIndexOf('\n') + 1;
                const spacesToRemove = beforeCursor.substring(lineStart).match(/^ {1,4}/);

                if (spacesToRemove && spacesToRemove[0].length > 0) {
                    const newValue = beforeCursor.substring(0, lineStart) +
                        beforeCursor.substring(lineStart).replace(/^ {1,4}/, '') +
                        afterCursor;
                    setInternalValue(newValue);
                    // Adjust the cursor position after removing spaces.
                    cursorPositionRef.current = cursorPosition - spacesToRemove[0].length;
                }
            } else {
                // For Tab, insert 4 spaces at cursor.
                const newValue = beforeCursor + '    ' + afterCursor;
                setInternalValue(newValue);
                // Adjust the cursor position after adding spaces.
                cursorPositionRef.current = cursorPosition + 4;
            }
        }
        if (event.key === 'Home' || event.key === 'End') {
            event.preventDefault();
            const textArea = event.target as HTMLTextAreaElement;
            const cursorPosition = textArea.selectionStart;

            if (event.key === 'Home') {
                if (event.ctrlKey || event.metaKey) {
                    // Move cursor to the start of the text area.
                    textArea.selectionStart = textArea.selectionEnd = 0;
                } else {
                    // Move cursor to the start of the current line.
                    const lineStart = textArea.value.substring(0, cursorPosition).lastIndexOf('\n') + 1;
                    textArea.selectionStart = textArea.selectionEnd = lineStart;
                }
            } else { // event.key === 'End'
                if (event.ctrlKey || event.metaKey) {
                    // Move cursor to the end of the text area.
                    textArea.selectionStart = textArea.selectionEnd = textArea.value.length;
                } else {
                    // Move cursor to the end of the current line.
                    const lineEnd = textArea.value.indexOf('\n', cursorPosition);
                    textArea.selectionStart = textArea.selectionEnd = lineEnd > -1 ? lineEnd : textArea.value.length;
                }
            }
        }

        if (event.key === '`' && !event.ctrlKey && !event.metaKey) {
            const cursorPos = event.currentTarget.selectionStart;
            const beforeCursor = value.substring(0, cursorPos);
            const afterCursor = value.substring(cursorPos);
            // Check if the last two characters before the cursor are backticks
            if (beforeCursor === "``" || beforeCursor.endsWith('\n``')) {
                event.preventDefault();
                const newValue = `${beforeCursor}\`\n\n\`\`\`${afterCursor}`;
                setInternalValue(newValue);
                // Set the new cursor position to be after the newline and before the closing backticks
                cursorPositionRef.current = cursorPos + 1; // +1 for the newline
            }
        }


        if (event.key === 'Enter') {
            event.preventDefault(); // Always prevent the default to insert our custom indentation.
            const textArea = event.target as HTMLTextAreaElement;

            if (event.ctrlKey || event.metaKey) {
                event.preventDefault();
                setValue(textArea.value);
                setInternalValue("");
                return;
            }

            const cursorPosition = textArea.selectionStart;
            const beforeCursor = value.substring(0, cursorPosition);
            const afterCursor = value.substring(cursorPosition);

            // Find the beginning of the current line.
            const currentLineStart = beforeCursor.lastIndexOf('\n') + 1;

            // Get the current line up to the cursor.
            const currentLineUpToCursor = beforeCursor.substring(currentLineStart);

            // Match the leading spaces or tabs in the current line.
            const matchIndent = currentLineUpToCursor.match(/^[ \t]*/);
            const indent = matchIndent ? matchIndent[0] : '';

            // Insert a newline and match the previous line's indentation.
            const newValue = beforeCursor + '\n' + indent + afterCursor;
            setInternalValue(newValue);

            // Move the cursor after the newline and indentation.
            const newCursorPosition = cursorPosition + indent.length + 1; // +1 for the newline character
            cursorPositionRef.current = newCursorPosition;
        }
    };

    const handleChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
        const newText = event.target.value;
        const selectionStart = event.target.selectionStart;

        // Replace left and right smart single and double quotes with straight quotes
        const newValue = newText.replace(/[\u2018\u2019\u201C\u201D]/g, (match) => {
            if (match === "\u2018" || match === "\u2019") return "'";
            else return '"';
        });

        // Adjust cursor position if needed for any replacements
        const diff = newValue.length - newText.length;

        if (newValue.length < newText.length) {
            cursorPositionRef.current = selectionStart + diff;
        } else {
            cursorPositionRef.current = selectionStart;
        }

        setInternalValue(newValue);
    };

    return (
        <textarea
            ref={textAreaRef}
            value={value}
            onKeyDown={handleKeyDown}
            onChange={handleChange}
            autoCapitalize="none"
            autoComplete="off"
            autoCorrect="off"
            placeholder="Type/paste request here..."
        />
    );
}
