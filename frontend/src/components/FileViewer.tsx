import React, {useEffect, useState} from "react";
import {Api, DirectoryNode, DirectoryTree, Media} from "@/api/api";
import styles from "@/components/FileViewer.module.css";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import {faFolder} from "@fortawesome/free-solid-svg-icons";
import {QueryState} from "@/hooks/useQueryState";
import {API_URL} from "@/global";
import {tree} from "next/dist/build/templates/app-page";
import {ViewType} from "@/components/GalleryStateSelector";
import Filter from "@/utility/Filter";
import Thumbnail from "@/components/Thumbnail";

interface FileViewerProps {
    api: Api
    filter: Filter,
    setGalleryState: (state: Partial<QueryState>) => void;
    setViewType: (viewType: ViewType) => void;
    media: Media[] | null,

    open: (media: Media) => void;
    selected: string[];
    select: (media: Media) => void;
    setLayout: (layout: Media[][] | null) => void;
}
export default function FileViewer({api, filter, setGalleryState, setViewType, media, open, select, selected, setLayout}: FileViewerProps) {
    const [directoryTree, setDirectoryTree] = useState<DirectoryTree | null>(null);

    const [currentPath, setCurrentPath] = useState<DirectoryNode[]>([]);

    const [loaded, setLoaded] = useState(false);

    function filterValid(tree: DirectoryTree){
        const path = filter.get("path", "%");
        const not_path = filter.get("path", "!%");
        // check if filter is a valid path
        if(!path || !not_path){
            return false;
        }
        // filter.path must be in format /dir1/dir2/%, filter.not_path must be in format /dir1/dir2/%/%
        if(!path.endsWith("/%") || !not_path.endsWith("/%/%")){
            return false;
        }
        // filter.path must be a prefix of filter.not_path
        if(!not_path.startsWith(path)){
            return false;
        }

        // filter.path must be a valid path in the tree

        // filter.path must start with the root
        if(!path.startsWith(tree.root.name)){
            return false;
        }

        const clean = path.slice(tree.root.name.length + 1).split("/");


        let curr = tree.root;

        let out = [curr];

        while(clean.length > 1){ // last element is %
            const next = clean.shift();
            const child = curr.children.find(c => c.name === next);
            if(!child){
                return false;
            }
            curr = child;
            out.push(curr);
        }

        return out;
    }

    useEffect(() => {
        api.directory_tree().then(tree => {
            tree.root.name = "";

            // condense tree
            let curr = tree.root;

            while (curr.children.length === 1 && curr.items === 0) {
                if(curr != tree.root){
                    tree.root.name = tree.root.name + "/" + curr.name;
                    tree.root.children = curr.children;
                }
                curr = curr.children[0];
            }

            setDirectoryTree(tree);
            setCurrentPath(filterValid(tree)  || [tree.root]);
        });
    }, []);

    useEffect(() => {
        // handle external changes to the filter
        // we only update when the filter is a valid path and it is different from the current path
        if(!directoryTree){
            return;
        }
        let path = filterValid(directoryTree);
        if(path){
            if(currentPath.map(node => node.name).join('/') !== path.map(node => node.name).join('/')){
                setCurrentPath(path);
            }
        } else {
            setViewType(ViewType.Gallery);
        }
    }, [filter]);

    useEffect(() => {
        if(!directoryTree){
            return;
        }

        const curr = currentPath.map(node => node.name).join('/');

        const path = curr + "/%";
        const not_path = curr + "/%/%";

        const filterPath = filter.get("path", "%");
        const filterNotPath = filter.get("path", "!%");

        if (filterPath === path && filterNotPath === not_path) {
            return;
        }

        const newFilter = filter.clone();

        newFilter.set("path", "%", path);
        newFilter.set("path", "!%", not_path);

        setGalleryState({
            filter: newFilter
        })
    }, [currentPath]);

    useEffect(() => {
        setLoaded(true);
        setLayout(null);
    }, [media]);

    const divs = [];

    if(directoryTree){
        let path = [];

        while (currentPath.length > path.length) {
            let current: DirectoryNode = currentPath[path.length];
            path.push(current);

            let currPath = [...path];

            divs.push(
                <div key={current.name} className={styles.dirContainer} onClick={(e) => {
                    if(e.target === e.currentTarget){
                        setLoaded(false);
                        setCurrentPath(currPath)
                    }
                }}>
                    {current.children.map(child => (
                        <div key={child.name} onClick={() => {
                            setLoaded(false);
                            setCurrentPath([...currPath, child])}
                        }  className={`${styles.dir} ${currentPath.includes(child) && styles.selected}`}>
                            <FontAwesomeIcon className={styles.folderIcon} icon={faFolder}/>
                            <div className={styles.fileName}>{child.name} ({child.items} media)</div>
                        </div>
                    ))}
                    {loaded && media && currentPath.length === path.length && (
                        media.map(m => (
                            <div key={m.uuid}
                                 className={`${styles.media} ${selected.includes(m.uuid) && styles.selected}`}
                                 onDragStart={(e) => {
                                     if (!selected.includes(m.uuid)) {
                                         select(m);
                                         e.dataTransfer.setData('text/json', JSON.stringify({selected: [m.uuid]}));
                                     } else {
                                         e.dataTransfer.setData('text/json', JSON.stringify({selected}));
                                     }
                                 }}
                                 draggable={true}
                                 onDoubleClick={() => open(m)}
                                 onMouseUp={() => select(m)}
                            >
                                <div className={styles.imageWrapper}>
                                    <Thumbnail media={m} draggable={false} className={styles.image}/>
                                </div>
                                <div className={styles.fileName}>{m.name}</div>
                            </div>
                        ))
                    )}
                </div>
            )
        }

    }

    return (
        <div className={styles.wrapper}>
        <div className={styles.pathHeader}>
                {currentPath.map((node, i) => (
                    <span key={node.name} onClick={() => setCurrentPath(currentPath.slice(0, i + 1))}>
                            <span className={styles.pathPart}>{node.name}</span>
                            <span>/</span>
                        </span>
                ))}
            </div>
            <div className={styles.browser}>
                {divs}
            </div>
        </div>
    )
}