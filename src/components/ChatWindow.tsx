// External modules
import React, {useCallback, useEffect, useRef, useState} from 'react';
import classNames from 'classnames';
import {invoke} from "@tauri-apps/api/tauri";
import {dialog} from "@tauri-apps/api";
import {emit, listen, UnlistenFn} from "@tauri-apps/api/event";
import Markdown from "react-markdown";
import {v4 as uuid4} from "uuid";


// Components
import {Loading} from "./Loading.tsx";
import {CustomTextArea} from "./CustomTextArea.tsx";
import {MemoizedCodeBlock} from "./CodeBlockComponent.tsx";
import {MaterialIcon} from "./MaterialIcon.tsx";

// Assets
import logo from "../assets/ca-logo.svg";
import {useScrollToBottom} from "../hooks/useScrollToBottom.tsx";
import {Message} from "../types";
import {updateMessages} from "../helpers/messageStreaming.ts";

function saveModelState(modelState: boolean) {
    localStorage.setItem("modelState", String(modelState));
}

function loadModelState() {
    const got = localStorage.getItem("modelState");
    if (got) {
        return JSON.parse(got)
    }
    return true
}


function errorDialog(err: Error, action?: string) {
    dialog.message(`ERROR: ${err}${action ? " from " + action : ""} `, {
        title: "Error",
        type: "error"
    }).catch(err => console.error(`ERROR: updable to display dialog: ${err}`));
}

export const ChatWindow: React.FC = () => {
    const [messages, setMessages] = useState<Message[]>([]);
    const [inputValue, setInputValue] = useState<string>('');
    const [isThinking, setIsThinking] = useState(false);
    const [resetKey, setResetKey] = useState(false)
    const [lowCost, setLowCost] = useState(loadModelState());

    const [messageQueue, setMessageQueue] = useState<string[]>([])

    function enqueueMessagePart(part: string) {
        setMessageQueue(prev => [...prev, part])
    }

    const dequeueMessagePart = useCallback(() => {
        if (!messageQueue.length) {
            return null
        } // Check for an empty queue
        const part = messageQueue[0]; // Get the first part of the queue
        setMessageQueue(queue => queue.slice(1)); // Remove the first part
        return part;
    }, [messageQueue, messages]);

    useEffect(() => {
        const part = dequeueMessagePart();
        if (!part) {
            return;
        }

        const updatedMessages = updateMessages(messages, part, "assistant");
        setMessages(updatedMessages);
    }, [dequeueMessagePart, messages]);

    const unlisten = useRef<Promise<UnlistenFn> | null>(null)

    function emitCancelStream() {
        return emit("cancel-stream");
    }

    async function stopListening() {
        await emitCancelStream()
        if (unlisten.current) {
            const dispose = await unlisten.current;
            dispose();
        } else {
            return async function () {
            }
        }
    }

    function startListening() {
        function start() {
            unlisten.current = listen("chat-message", ({payload}) => {
                setIsThinking(false);
                enqueueMessagePart(payload as string);
            });
        }

        if (unlisten.current) {
            stopListening().then(start)
            return;
        }
        start();
    }

    useEffect(() => {
        const errorListener = listen("stream-error", ({payload}) => {
            errorDialog(payload as Error, "streamingMessage")
        })
        return () => {
            errorListener.then(dispose => dispose())
        }
    }, []);

    useEffect(() => {
        startListening()
        return () => {
            stopListening().catch((e) => errorDialog(e, "chatMessageListener"))
        };
    }, []);
    const containerRef = useRef<HTMLDivElement>(null)

    const bottom = useScrollToBottom(messages.length);

    async function getHistory() {
        const history = await invoke("get_history") as { history: { role: string, content: string }[] };
        setMessages(history.history.filter(h => h.role !== "system").map(h => ({
            id: `${uuid4()}-${h.role}-message`,
            content: h.content,
            role: h.role as "user" | "assistant"
        })));
    }

    useEffect(() => {
        getHistory().catch(err => errorDialog(err, "getHistory"))
    }, []);

    useEffect(() => {
        saveModelState(lowCost)
    }, [lowCost]);

    const handleSubmit = useCallback(async (e?: React.FormEvent) => {
        e?.preventDefault();
        if (!inputValue.trim()) {
            return;
        }

        setResetKey(prev => !prev);
        setInputValue('');

        stopListening().then(() => {
            setIsThinking(true);
            const newMessage: Message = {
                id: `${uuid4()}-user-message`,
                content: inputValue,
                role: "user"
            };
            setMessages(prevMessages => [...prevMessages, newMessage]);
            startListening();
        }).then(() =>
            invoke("prompt", {markdown: inputValue, lowCost}).catch(async (err) => {
                setIsThinking(false);
                await errorDialog(err, "submitRequest");
            }));
    }, [inputValue, messages]);

    const newChat = useCallback(async () => {
        const confirmed = await dialog.confirm("Are you sure you want to clear the current chat?\n\nIt will be backed up to\n~/.coding-assistant-history", {
            title: "Confirm",
            type: "warning"
        });
        if (!confirmed) return;
        setIsThinking(false)
        stopListening().then(() =>
            invoke("clear_history").then(() => {
                getHistory()
                startListening();
                setHasScrollbar(false)
            }).catch(async (error) => {
                await errorDialog(error, "newChat");
            }).finally(startListening))
    }, []);

    useEffect(() => {
        if (inputValue.trim().length)
            handleSubmit().catch(errorDialog)
    }, [inputValue]);


    const [hasScrollbar, setHasScrollbar] = useState(false);
    useEffect(() => {
        if (hasScrollbar) return;
        const container = containerRef.current;

        // Check if the container has a vertical scrollbar
        const checkScrollbar = () => {
            if (container) {
                setHasScrollbar(container.scrollHeight > container.clientHeight);
            }
        };

        // Call checkScrollbar whenever the container is updated
        checkScrollbar();

        // Event listener for checking when the content changes
        const handleContentChange = () => {
            checkScrollbar();
        };

        if (container) {
            container.addEventListener('DOMSubtreeModified', handleContentChange);
        }

        return () => {
            if (container) {
                container.removeEventListener('DOMSubtreeModified', handleContentChange);
            }
        };
    }, [hasScrollbar]);

    return (
        <div ref={containerRef} className={classNames("container", {
            "faded-logo-background": !messages.length,
            "has-scrollbar": hasScrollbar
        })} style={!messages.length ? {backgroundImage: `url("${logo}")`} : undefined}>
            <div className="messages">
                {messages.map(message => (
                    <div key={message.id} className={classNames("message", {
                        'sent': message.role === "user",
                        'received': message.role === "assistant"
                    })}>
                        <div
                            className="message-role">{message.role.substring(0, 1).toUpperCase()}{message.role.substring(1)}</div>
                        <div className="message-content"><Markdown
                            children={message.content}
                            components={{
                                code(props) {
                                    return <MemoizedCodeBlock {...props} />
                                }
                            }}
                        /></div>
                    </div>
                ))}
                {isThinking && <Loading/>}
                {bottom}
            </div>

            <form onSubmit={handleSubmit} autoCapitalize="none" autoCorrect="off" autoComplete="off">
                <CustomTextArea setValue={setInputValue} key={`text-area-${resetKey}`}/>
                <div className="action-buttons">
                    <div className="top-left-actions">
                        <button onClick={() => {
                            setLowCost(!lowCost)
                            // The state below is one cycle behind:
                            if (lowCost) {
                                dialog.message('Now using GPT4\n\nBeware of increased costs when using this model')
                            }
                        }}><MaterialIcon icon="smart_toy" className={classNames({highlight: !lowCost})}/>{lowCost ? "GPT 3" : "GPT 4"}</button>
                    </div>
                    {!!messages.length && <div className="top-right-actions">
                        <button onClick={newChat}><MaterialIcon icon="delete_sweep"/>New Session</button>
                    </div>}
                </div>
            </form>
        </div>
    );
};
