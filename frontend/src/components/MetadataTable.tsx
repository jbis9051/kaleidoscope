import styles from "./MetadataTable.module.css";

export default function MetadataTable({metadata}: { metadata: Record<string, string> }) {
    return (
        <table className={styles.container}>
            <tbody>
            {Object.entries(metadata).map(([key, value]) => (
                <tr key={key}>
                    <td className={styles.key}>{key}</td>
                    <td className={styles.value}>{value}</td>
                </tr>
            ))}
            </tbody>
        </table>
    );
}