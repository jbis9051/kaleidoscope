export type OrderByColumns = string;
export type Value = any;

type FilterType = { [key: string]: [string, Value][] };

export default class Filter {
    filter: FilterType = {};

    static fromString(filter: string): Filter {
        const f = new Filter();

        if (filter.length === 0) {
            return f;
        }

        const parts = filter.split(' ');
        for (const part of parts) {
            const [key, rest] = part.split(':');

            if (!rest) {
                throw new Error(`invalid filter ${part}`);
            }

            const match = rest.match(/([=<>!%]+)(.+)/);
            if (!match || match.length !== 3) {
                throw new Error(`invalid filter ${part}`);
            }

            const [op, value] = match.splice(1);

            f.set(key, op, value);
        }

        return f;
    }

    toFilterString(): string {
        // <key>:<op>value> <key>:<op><value>
        return Filter.stringify(this.filter);
    }

    private static stringify(filters: FilterType) {
        return Object.entries(filters).map(([key, values]) => {
            return values.map(([op, value]) => `${key}:${op}${value}`).join(' ');
        }).join(' ');
    }

    toMediaQuery(order_by: OrderByColumns, asc: boolean, limit: number, page: number) {
        const filterString = this.toFilterString();
        return `${filterString} order_by:=${order_by} asc:=${asc} limit:=${limit} page:=${page}`;
    }

    static empty() {
        return new Filter();
    }

    get(key: string, op: string): Value | null {
        const values = this.filter[key];
        if (!values) {
            return null;
        }

        const value = values.find(([op_, _]) => op_ === op);
        if (!value) {
            return null;
        }

        return value[1];
    }

    set(key: string, op: string, value: Value) {
        if (!this.filter[key]) {
            this.filter[key] = [];
        }
        this.filter[key] = this.filter[key].filter(([op_, _]) => op_ !== op);
        this.filter[key].push([op, value]);
        return this;
    }

    clone() {
       return Filter.fromString(this.toFilterString());
    }
}