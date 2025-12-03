import { useWindowSize } from "@vueuse/core";

export const useFormat = () => {
    const { width } = useWindowSize();

    const formatHash = (address: string) => {
        if (!address || address.length <= 20) return address;

        const minWidth = 375;
        const maxWidth = 1920;
        const currentWidth = Math.max(minWidth, Math.min(width.value, maxWidth));
        const normalizedWidth = (currentWidth - minWidth) / (maxWidth - minWidth);
        const curve = Math.pow(normalizedWidth, 0.6);
        const startChars = Math.round(15 + curve * 20);
        const endChars = Math.round(15 + curve * 20);

        const totalChars = startChars + endChars + 3;
        if (totalChars >= address.length) return address;

        return `${address.slice(0, startChars)}...${address.slice(-endChars)}`;
    };

    const formatAddress = (address: string) => {
        if (!address || address.length <= 20) return address;
        return `${address.slice(0, 10)}...${address.slice(-10)}`;
    };

    const formatAmount = (value: number, precision: number = 4) => value.toFixed(precision).replace(/\.?0+$/, "");

    return {
        formatHash,
        formatAddress,
        formatAmount,
    };
};
