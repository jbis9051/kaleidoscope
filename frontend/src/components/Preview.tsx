import styles from "./Preview.module.css";
import MediaDisplay from "@/components/MediaDisplay";
import Transcript from "@/components/Transcript";
import React from "react";
import {Media, MediaExtra, MediaType, VisionOCRResult} from "@/api/api";
import useFitText from "use-fit-text";

interface PreviewProps {
    preview: Media,
    previewRef: React.RefObject<HTMLVideoElement | HTMLAudioElement>
    selectedMediaExtra: MediaExtra | null,
    onExit: () => void
}

function OCRText({ ocr }: { ocr: VisionOCRResult }) {
    const { fontSize, ref } = useFitText({ minFontSize: 0, maxFontSize: 1000 });

    return (
        <div
            ref={ref}
            className={styles.visionOCRResult}
            style={{
                left: `${ocr.origin_x * 100}%`,
                bottom: `${ocr.origin_y * 100}%`,
                width: `${ocr.size_width * 100}%`,
                height: `${ocr.size_height * 100}%`,
                fontSize,
            }}
        >
            {ocr.text}
        </div>
    );
}

export default function Preview({preview, previewRef, selectedMediaExtra, onExit}: PreviewProps) {
    const vision_ocr = selectedMediaExtra?.vision_ocr_result ? JSON.parse(selectedMediaExtra.vision_ocr_result) as VisionOCRResult[] : [];

    return (
        <div className={styles.previewWrapper}>
            <div className={styles.previewMedia}>
                <div className={styles.previewMediaWrapper}
                     style={preview.media_type === MediaType.Pdf ? {height: "100%"} : {}}>
                    {vision_ocr.map((ocr, index) => (
                        <OCRText key={index} ocr={ocr} />
                    ))}
                    <MediaDisplay media={preview} preferThumbnail={false}
                                  mediaRef={previewRef}
                                  objectProps={{className: styles.pdfObject}}
                                  audioProps={{className: styles.audioElement}}/>
                </div>
            </div>
            {selectedMediaExtra?.whisper_transcript &&
                <div className={styles.previewTranscript}>
                    <div className={styles.transcriptContent}>
                        <Transcript mediaRef={previewRef} transcript={selectedMediaExtra.whisper_transcript}/>
                    </div>
                </div>
            }
            <button onClick={onExit}>X</button>
        </div>
    );
}