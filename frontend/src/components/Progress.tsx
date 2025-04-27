import styles from './Progress.module.css'
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faHourglass} from "@fortawesome/free-solid-svg-icons";
import {useEffect, useState} from "react";
import {Api, QueueProgress} from "@/api/api";
import {capitalize, durationHumanReadable} from "@/utility/mediaMetadata";

function ProgressBar({total, progress}: { total: number, progress: number }) {
    return <div className={styles.progressBarContainer}>
        <div className={styles.progressBar}>
            <div className={styles.progress} style={{width: `${(progress / total) * 100}%`}}/>
        </div>
        <div className={styles.progressText}>
            ({progress}/{total})
        </div>
    </div>
}

export default function Progress({api}: { api: Api }) {
    const [visible, setVisible] = useState(false);
    const [progress, setProgress] = useState<QueueProgress | null>(null);

    useEffect(() => {
        let timeout: NodeJS.Timeout | null = null;

        function updateProgress() {
            timeout = null;
            api.queue_status().then(p => setProgress(p));
            if (visible) {
                timeout = setTimeout(updateProgress, 1000);
            }
        }

        if (visible) {
            updateProgress();
        } else if (timeout) {
            clearTimeout(timeout);
            setProgress(null);
        }

        return () => {
            if (timeout) {
                clearTimeout(timeout)
            }
        }
    }, [visible]);

    return (
        <div className={styles.container} onClick={() => setVisible(!visible)}>
            <FontAwesomeIcon icon={faHourglass}/>

            {visible && <div className={styles.progressIndicator}>
                <div>Status: {capitalize(progress?.status || 'Loading...')}</div>
                {(() => {
                    if (!progress) {
                        return null;
                    }
                    if (progress.status === "Starting" && progress.total > 0) {
                        return <ProgressBar total={progress.total} progress={0}/>
                    }
                    if (progress.status === "Done") {
                        return <div>Succeeded: {progress.Ok[0]}, Failed: {progress.Ok[1]}</div>;
                    }
                    if (progress.status === "Progress") {
                        return <>
                            <div>Task: {progress.task}, Took: {durationHumanReadable(progress.time * 1000)}</div>
                            {progress.error && <div>{progress.error}</div>}
                            <ProgressBar total={progress.total} progress={progress.index + 1}/>
                        </>
                    }
                })()}
            </div>
            }
        </div>
    )
}