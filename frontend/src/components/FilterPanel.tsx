import {useEffect, useRef, useState} from "react";
import styles from "@/components/FilterPanel.module.css";
import Filter from "@/utility/Filter";
import {Api, MediaQueryDescription} from "@/api/api";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faCircleCheck, faCircleXmark} from "@fortawesome/free-solid-svg-icons";

interface FilterPanelProps {
    api: Api;

    filter: Filter;

    trashEnabled: boolean;

    setFilter: (filter: Filter) => void;
    onTrash: () => void;
    onSave: () => void;
}

export default function FilterPanel({api, filter, trashEnabled, setFilter, onTrash, onSave}: FilterPanelProps) {
    const [filterInput, setFilterInput] = useState<string>("");

    const [description, setDescription] = useState<MediaQueryDescription | null>(null);
    const [suggestions, setSuggestions] = useState<string[]>([]);
    const [cursor, setCursor] = useState<number|null>(null);
    const [selectedSuggestion, setSelectedSuggestion] = useState<number>(0);

    const inputRef = useRef<HTMLInputElement>(null);

    useEffect(() => {
        api.info().then(info => {
            const desc = info.media_query;
            delete desc.fields["page"];
            delete desc.fields["asc"];
            delete desc.fields["limit"];
            delete desc.fields["order_by"];
            setDescription(info.media_query)
        });
    }, []);

    // update the filterInputs when the filter changes
    useEffect(() => {
        setFilterInput(filter.toFilterString());
    }, [filter])

    useEffect(() => {
        if(!description || cursor === null) {
            setSuggestions([]);
            return;
        }
        const before = filterInput.substring(0, cursor);
        const lastWhitespace = before.lastIndexOf(" ");
        const part = before.substring(lastWhitespace + 1);

        if(!part.includes(":")) {
            setSuggestions(Object.keys(description.fields).filter(f => f.startsWith(part)));
            return;
        }
        const [key, rest] = part.split(":");
        const dsl = description.fields[key];
        if(!dsl) {
            setSuggestions([])
            return;
        }
        const operators = description.dsl_types[dsl];
        const operator = operators.find(o => rest.startsWith(o));
        if(!operator) {
            setSuggestions(operators.filter(o => o.startsWith(rest)));
            return;
        }
        setSuggestions([]);
    }, [cursor]);

    useEffect(() => {
        if(selectedSuggestion > suggestions.length){
            setSelectedSuggestion(0);
        }
    }, [suggestions]);

    function selectSuggestion(suggestion: string) {
        if(cursor === null || !inputRef.current) {
            return;
        }
        setSelectedSuggestion(0);
        const before = filterInput.substring(0, cursor);
        const after = filterInput.substring(cursor);

        const lastWhitespace = before.lastIndexOf(" ");
        const part = before.substring(lastWhitespace + 1);
        const beforePart = filterInput.substring(0, lastWhitespace+1);

        if(!part.includes(":")) { // suggestion is a key
            const input = `${beforePart}${suggestion}:${after.length === 0 || after.startsWith(" ") ? "" : " "}${after}`;
            setFilterInput(input);
            const newCursor = beforePart.length + suggestion.length + 1;
            setCursor(newCursor);
            requestAnimationFrame(() => inputRef.current?.setSelectionRange(newCursor, newCursor));
            return;
        }
       // suggestion is an operator
        const [key, _rest] = part.split(":");
        const input = `${beforePart}${key}:${suggestion}${after.length === 0 || after.startsWith(" ") ? "" : " "}${after}`;
        setFilterInput(input);
        const newCursor = beforePart.length + key.length + 1 + suggestion.length;
        setCursor(newCursor);
        requestAnimationFrame(() => inputRef.current?.setSelectionRange(newCursor, newCursor));
        return;

    }

    let filterError = null;

    try {
        Filter.fromString(filterInput);
    } catch (e: any) {
        filterError = e.message;
    }


    return (
        <div className={styles.filterPanel}>
            <div className={`${styles.errorPreview} ${filterError ? styles.error : ""}`}>
                <div className={styles.errorIcon}>
                    <FontAwesomeIcon className={styles.icon} icon={filterError ? faCircleXmark : faCircleCheck} />
                    {filterError && <div className={styles.errorMessage}>
                        {filterError}
                    </div>}
                </div>
            </div>
            <div className={`${styles.filter} ${filterError ? styles.error : ""}`}>
                <input
                    ref={inputRef}
                    placeholder={"Filter"}
                    value={filterInput}
                    onChange={(e) => {
                        setFilterInput(e.target.value);
                        setCursor(e.target.selectionStart);
                    }}
                    onClick={(e) => {
                        // @ts-ignore
                        setCursor(e.target.selectionStart);
                    }}
                    onKeyUp={(e) => {
                        // @ts-ignore
                        setCursor(e.target.selectionStart);
                    }}
                    onKeyDown={(e) => {
                        if(e.key === "ArrowUp") {
                            e.preventDefault();
                            setSelectedSuggestion((selectedSuggestion - 1 + suggestions.length) % suggestions.length);
                        }
                        if(e.key === "ArrowDown") {
                            e.preventDefault();
                            setSelectedSuggestion((selectedSuggestion + 1) % suggestions.length);
                        }
                        if(e.key === "Enter") {
                            e.preventDefault();
                            if(selectedSuggestion >= 0 && selectedSuggestion < suggestions.length) {
                                selectSuggestion(suggestions[selectedSuggestion]);
                            }
                        }
                    }}
                    onBlur={() => {
                        setCursor(null);
                    }}
                />
            </div>
            <div className={styles.actions}>
                <div>
                    <button onClick={onSave}>Save</button>
                    <button disabled={!trashEnabled} onClick={onTrash}>Trash</button>
                    <button onClick={() => setFilter(Filter.fromString(filterInput)) }>Filter</button>
                </div>
            </div>
            {
                suggestions.length > 0 &&
                <div className={styles.suggestions}>
                    {suggestions.map((s,i) =>
                        <div
                            onMouseDown={(e) => {
                                e.preventDefault();
                                selectSuggestion(s);
                            } }
                            key={s} className={`${styles.suggestion} ${i===selectedSuggestion && styles.selected}`}>{s}</div>)}
                </div>
            }

        </div>
    )
}