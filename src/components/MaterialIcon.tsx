import 'material-symbols/rounded.css';
import { MaterialSymbol } from 'material-symbols';
import classNames from 'classnames';

const HEAVY = 700;
const NORMAL = 400;
const LIGHT = 100;

interface Props {
    icon: MaterialSymbol;
    light?: boolean;
    heavy?: boolean;
    filled?: boolean;
    className?: string;
}

export function MaterialIcon({ icon, light, heavy, filled, className }: Props) {
    const fill = filled ? 1 : 0;
    const weight = heavy ? HEAVY : light ? LIGHT : NORMAL;

    const style = {
        fontVariationSettings: `
    'FILL' ${fill},
    'wght' ${weight},
    'GRAD' 0,
    'opsz' 24`,
    };

    return (
        <span
            className={classNames('material-symbols-rounded', className)}
            style={style}
        >
      {icon}
    </span>
    );
}
