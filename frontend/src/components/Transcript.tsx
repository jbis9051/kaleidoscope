import styles from "./Transcript.module.css";
import React, {useEffect, useState} from "react";
import {durationHumanReadable} from "@/utility/mediaMetadata";

type TranscriptElement = [number, number, string]

export default function Transcript({transcript: transcriptText, mediaRef}: { transcript: string, mediaRef?: React.RefObject<HTMLVideoElement | HTMLAudioElement> }) {
    const [transcript, setTranscript] = useState<TranscriptElement[]>([]);
    const [currentTime, setCurrentTime] = useState<number | null>(null);

    useEffect(() => {
        setTranscript(JSON.parse(transcriptText))
    }, [transcriptText]);

    useEffect(() => {
        function timeupdate(){
            if (mediaRef?.current) {
                setCurrentTime(mediaRef.current.currentTime);
            }
        }

        mediaRef?.current?.addEventListener("timeupdate", timeupdate);
        return () => {
            mediaRef?.current?.removeEventListener("timeupdate", timeupdate);
        }
    }, [mediaRef]);

    const displayHours = transcript.every(e => e[0] >= 3600);

    return (
        <div className={styles.container}>
            {transcript.map((element: TranscriptElement, index: number) => {
                const [start, end, text] = element;

                let startDisplay = durationHumanReadable(start * 1000);
                let endDisplay = durationHumanReadable(end * 1000);

                if (!displayHours) {
                    startDisplay = startDisplay.substring(3);
                    endDisplay = endDisplay.substring(3);
                }

                return (
                    <div onClick={() => {

                        if (mediaRef?.current) {
                            mediaRef.current.currentTime = start;
                        }

                    }} key={index} className={`${styles.transcript} ${currentTime && currentTime >= start && currentTime <= end ? styles.active : ""}`}>
                        <span className={styles.time}>{startDisplay}</span>
                        <span className={styles.text}>{text}</span>
                    </div>
                );
            })}
        </div>
    );
}