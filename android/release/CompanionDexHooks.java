import java.io.File;
import java.util.*;
import org.jf.dexlib2.*;
import org.jf.dexlib2.iface.*;
import org.jf.dexlib2.immutable.*;
import org.jf.dexlib2.immutable.reference.ImmutableMethodReference;
import org.jf.dexlib2.builder.*;
import org.jf.dexlib2.builder.instruction.*;

/** Rebuild DEX pools after adding two source-built companion hooks. */
public final class CompanionDexHooks {
    public static void main(String[] args) throws Exception {
        DexFile dex=DexFileFactory.loadDexFile(new File(args[0]),Opcodes.forApi(26));
        List<ClassDef> classes=new ArrayList<>(); int hooks=0;
        for(ClassDef cls:dex.getClasses()) {
            if(!cls.getType().equals("Lorg/rerac/android/WeaponWheel;")){classes.add(cls);continue;}
            List<Method> methods=new ArrayList<>();
            for(Method method:cls.getMethods()) {
                boolean draw=method.getName().equals("onDraw")&&method.getParameterTypes().equals(Arrays.asList("Landroid/graphics/Canvas;"));
                boolean touch=method.getName().equals("onTouchEvent")&&method.getParameterTypes().equals(Arrays.asList("Landroid/view/MotionEvent;"));
                if(!draw&&!touch){methods.add(method);continue;}
                MutableMethodImplementation code=new MutableMethodImplementation(method.getImplementation());
                Label original=code.newLabelForIndex(0);
                int self=code.getRegisterCount()-2;
                code.addInstruction(0,new BuilderInstruction3rc(Opcode.INVOKE_STATIC_RANGE,self,2,
                    new ImmutableMethodReference("Lorg/rerac/android/CompanionMenu;",draw?"draw":"touch",
                    Arrays.asList("Landroid/view/View;",draw?"Landroid/graphics/Canvas;":"Landroid/view/MotionEvent;"),"Z")));
                code.addInstruction(1,new BuilderInstruction11x(Opcode.MOVE_RESULT,0));
                code.addInstruction(2,new BuilderInstruction21t(Opcode.IF_EQZ,0,original));
                code.addInstruction(3,draw?new BuilderInstruction10x(Opcode.RETURN_VOID):new BuilderInstruction11x(Opcode.RETURN,0));
                methods.add(new ImmutableMethod(method.getDefiningClass(),method.getName(),method.getParameters(),method.getReturnType(),method.getAccessFlags(),method.getAnnotations(),method.getHiddenApiRestrictions(),code));
                hooks++;
            }
            classes.add(new ImmutableClassDef(cls.getType(),cls.getAccessFlags(),cls.getSuperclass(),cls.getInterfaces(),cls.getSourceFile(),cls.getAnnotations(),cls.getFields(),methods));
        }
        if(hooks!=2)throw new IllegalStateException("Expected exactly two retained companion hooks; got "+hooks);
        DexFileFactory.writeDexFile(args[1],new ImmutableDexFile(Opcodes.forApi(26),classes));
    }
}
