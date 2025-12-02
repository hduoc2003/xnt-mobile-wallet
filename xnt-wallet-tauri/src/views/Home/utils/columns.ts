export const columns = [
    {
        title: 'UTXO Hash',
        dataIndex: 'utxoHash',
        key: 'utxoHash',
        width: '60%',
        ellipsis: true,
        customRender: ({ text }: any) => {
            if (!text) return 'N/A'
            const str = typeof text === 'string' ? text : JSON.stringify(text)
            return str.length > 20 ? `${str.slice(0, 5)}...${str.slice(-5)}` : str
        },
    },
    {
        title: 'Amount (XNT)',
        dataIndex: 'amount',
        key: 'amount',
        width: '20%',
        customRender: ({ record }: any) => {
            const amount = record.amount || record.value || 0
            return parseFloat(amount).toFixed(8)
        },
    },
    {
        title: 'Block Height',
        dataIndex: 'blockHeight',
        key: 'blockHeight',
        width: '20%',
        customRender: ({ text }: any) => text || 'N/A',
    },
]
