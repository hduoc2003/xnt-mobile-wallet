// Button Component Props
export type ButtonType = 'default' | 'primary' | 'dashed' | 'link' | 'text'
export type ButtonSize = 'small' | 'medium' | 'large'

export interface ButtonProps {
    type?: ButtonType
    size?: ButtonSize
    block?: boolean
    disabled?: boolean
    loading?: boolean
    htmlType?: 'button' | 'submit' | 'reset'
}

// Icon Component Props
export interface IconProps {
    name?: string
    size?: number | string
    color?: string
    class?: string
    icon: string
}
