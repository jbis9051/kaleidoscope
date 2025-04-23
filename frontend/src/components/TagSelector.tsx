import styles from "@/components/TagSelector.module.css";
import {Api, MediaTagIndex} from "@/api/api";
import React from "react";
import Filter from "@/utility/Filter";

export interface TagSelectorProps {
    tags: MediaTagIndex[];
    filter: Filter,
    setFilter: (newFilter: Filter) => void
    deleteTag: (tag: string) => void,
}


export default function TagSelector({tags, filter, setFilter, deleteTag}: TagSelectorProps) {
    const currentTags = filter.getAll<string>("tag", "=");

    function toggleTag(tag: string){
        const newFilter = filter.clone();
        if(currentTags.includes(tag)){
            newFilter.remove("tag", "=", tag);
        } else {
            newFilter.add("tag", "=", tag);
        }
        setFilter(newFilter);
    }

    return <div className={styles.tagSelector}>
        <div className={styles.tagHeader}>
            <div className={styles.tagTitle}>Tags</div>
            <div className={styles.albumControls}>
                <button onClick={() => {
                    deleteTag(currentTags[0]);
                    toggleTag(currentTags[0]);
                }} disabled={currentTags.length !== 1}>Delete All</button>
            </div>
        </div>
        <div className={styles.tagContainer}>
            <div className={styles.tags}>
                {tags.map((tagIndex) => (
                    <div
                        className={`${styles.tag} ${currentTags.includes(tagIndex.tag) && styles.selected}`}
                        key={tagIndex.id}
                        onClick={() => toggleTag(tagIndex.tag)}>
                        <span className={styles.name}>{tagIndex.tag}</span>
                        <span className={styles.count}>({tagIndex.media_count})</span>
                    </div>
                ))}
            </div>
        </div>
    </div>
}