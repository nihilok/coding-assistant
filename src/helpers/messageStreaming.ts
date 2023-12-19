import {Message} from "../types";
import {v4 as uuid4} from "uuid";

export function updateMessages(
    messages: Message[],
    newContent: string,
    role: "user" | "assistant"
): Message[] {
    let updatedMessages = [...messages];
    let messageToUpdateIndex: number;

    if (updatedMessages.length > 0 && updatedMessages[updatedMessages.length - 1].role === role) {
        messageToUpdateIndex = updatedMessages.length - 1;
    } else {
        const newMessage: Message = {
            id: `${uuid4()}-${role}-message`,
            role: role,
            content: "..."
        };
        updatedMessages.push(newMessage);
        messageToUpdateIndex = updatedMessages.length - 1;
    }

    updatedMessages[messageToUpdateIndex] = {
        ...updatedMessages[messageToUpdateIndex],
        content: updatedMessages[messageToUpdateIndex].content === "..." ? newContent : updatedMessages[messageToUpdateIndex].content + newContent,
    };

    return updatedMessages;
}