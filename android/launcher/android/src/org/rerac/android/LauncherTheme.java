package org.rerac.android;

import android.content.Context;
import android.content.res.ColorStateList;
import android.graphics.*;
import android.graphics.drawable.Drawable;
import android.os.Build;
import android.view.Gravity;
import android.view.View;
import android.widget.*;

/** Static, resolution-independent Gadgetron-inspired panels. No frame loop or game assets. */
final class LauncherTheme {
    static final String SECTION = "org.rerac.android.theme-section";
    private LauncherTheme() {}
    static int dp(Context c, float n) { return LauncherAppearance.dp(c, n); }
    static ColorStateList tint() {
        return new ColorStateList(new int[][]{{-android.R.attr.state_enabled},
                {android.R.attr.state_checked}, {android.R.attr.state_pressed},
                {android.R.attr.state_focused}, {}},
                new int[]{0xff697184, LauncherAppearance.YELLOW, LauncherAppearance.YELLOW,
                        LauncherAppearance.YELLOW, LauncherAppearance.BLUE});
    }
    static void icon(TextView v, String name) {
        v.setCompoundDrawablesRelativeWithIntrinsicBounds(new Symbol(v.getContext(), name), null, null, null);
        v.setCompoundDrawablePadding(dp(v.getContext(), 12));
    }
    static void choice(CompoundButton v) {
        Context c=v.getContext();
        LauncherAppearance.text(v, 16, LauncherAppearance.BLUE);
        v.setTextColor(tint()); v.setButtonTintList(tint());
        v.setMinHeight(dp(c, 52));v.setMinimumHeight(dp(c,52));
        v.setPadding(dp(c,12),dp(c,10),dp(c,12),dp(c,10));
        v.setBackgroundTintList(null);v.setBackground(new Plate(c,1));
        v.setStateListAnimator(null);v.setFocusable(true);
        if(v instanceof Switch){Switch s=(Switch)v;s.setThumbTintList(tint());
            s.setTrackTintList(ColorStateList.valueOf(0xff44557b));s.setSwitchPadding(dp(c,16));}
    }
    static TextView eyebrow(Context c,String text) {
        TextView v=LauncherAppearance.label(c,text,11,LauncherAppearance.MUTED);
        v.setTypeface(Typeface.create("sans-serif-condensed",Typeface.BOLD));
        v.setLetterSpacing(.14f);return v;
    }
    static LinearLayout card(Context c,String title,String icon) {
        LinearLayout p=LauncherAppearance.panel(c,title);p.setTag(SECTION);
        if(icon!=null)icon((TextView)p.getChildAt(0),icon);return p;
    }
    static void status(TextView v) {
        v.setPadding(dp(v.getContext(),14),dp(v.getContext(),12),dp(v.getContext(),14),dp(v.getContext(),12));
        v.setBackground(new Plate(v.getContext(),0));
        v.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
    }
    static final class Plate extends Drawable {
        final Paint p=new Paint(Paint.ANTI_ALIAS_FLAG);
        final Path shape=new Path();final int role;final float unit;
        int opacity=255;Shader gradient;
        Plate(Context c,int role){this.role=role;unit=c.getResources().getDisplayMetrics().density;}
        @Override public boolean isStateful(){return role==1||role==2;}
        @Override protected boolean onStateChange(int[] state){invalidateSelf();return true;}
        @Override protected void onBoundsChange(Rect r){
            int top=role==3?0xff101b32:role==2?0xff303328:0xff1b2841;
            int bottom=role==3?0xff050912:role==2?0xff171c24:0xff0c1425;
            gradient=new LinearGradient(r.left,r.top,r.right,r.bottom,top,bottom,Shader.TileMode.CLAMP);
        }
        boolean has(int value){for(int s:getState())if(s==value)return true;return false;}
        void color(int color,int a){p.setShader(null);p.setColor(color);p.setAlpha(a*opacity/255);}
        @Override public void draw(Canvas c){
            Rect r=getBounds();boolean enabled=!isStateful()||has(android.R.attr.state_enabled);
            boolean active=enabled&&(has(android.R.attr.state_pressed)||has(android.R.attr.state_focused)||has(android.R.attr.state_checked)||has(android.R.attr.state_selected));
            float u=Math.max(1,unit),cut=Math.min(10*u,r.height()/5f),in=u;
            int checkpoint=c.save();
            if(role==3){
                p.setStyle(Paint.Style.FILL);p.setShader(gradient);p.setAlpha(opacity);c.drawRect(r,p);p.setShader(null);
                color(0xff426695,20);p.setStrokeWidth(u);p.setStyle(Paint.Style.STROKE);
                float cx=r.right-r.width()*.12f,cy=r.top+r.height()*.29f;
                for(int i=0;i<4;i++)c.drawCircle(cx,cy,(110+62*i)*u,p);
                p.setStyle(Paint.Style.FILL);color(0xff3e5789,20);
                shape.reset();shape.moveTo(r.right-r.width()*.22f,r.top);shape.lineTo(r.right,r.top);
                shape.lineTo(r.right-r.width()*.38f,r.bottom);shape.lineTo(r.right-r.width()*.48f,r.bottom);shape.close();c.drawPath(shape,p);
                color(0xff000000,30);for(float y=r.top;y<r.bottom;y+=4*u)c.drawRect(r.left,y,r.right,y+u,p);
            }else{
                shape.reset();shape.moveTo(r.left+in,r.top+in);shape.lineTo(r.right-cut,r.top+in);
                shape.lineTo(r.right-in,r.top+cut);shape.lineTo(r.right-in,r.bottom-in);
                shape.lineTo(r.left+cut,r.bottom-in);shape.lineTo(r.left+in,r.bottom-cut);shape.close();
                c.clipPath(shape);p.setStyle(Paint.Style.FILL);p.setShader(gradient);p.setAlpha(opacity);c.drawPath(shape,p);p.setShader(null);
                if(!enabled){color(0xff080e19,160);c.drawRect(r,p);}
                if(active){color(0xff728342,45);c.drawRect(r,p);}
                color(0xff000000,22);for(float y=r.top+3*u;y<r.bottom;y+=4*u)c.drawRect(r.left,y,r.right,y+u*.5f,p);
                p.setStyle(Paint.Style.STROKE);p.setStrokeWidth(u);
                color(active?LauncherAppearance.YELLOW:role==2&&enabled?0xff92994e:0xff435778,255);c.drawPath(shape,p);
                p.setStyle(Paint.Style.FILL);color(active?LauncherAppearance.YELLOW:role==2&&enabled?0xffc7c962:0xff6985b5,enabled?220:90);
                c.drawRect(r.left+2*u,r.top+2*u,r.left+Math.min(r.width()*.2f,68*u),r.top+3*u,p);
                if(active)c.drawRect(r.left+u,r.top+8*u,r.left+4*u,r.bottom-8*u,p);
                color(0xff9cb0d2,65);c.drawCircle(r.right-8*u,r.bottom-8*u,1.5f*u,p);
                c.drawCircle(r.left+8*u,r.top+8*u,1.5f*u,p);
            }
            c.restoreToCount(checkpoint);
        }
        @Override public void setAlpha(int a){opacity=Math.max(0,Math.min(255,a));invalidateSelf();}
        @Override public void setColorFilter(ColorFilter f){p.setColorFilter(f);invalidateSelf();}
        @Override public int getOpacity(){return PixelFormat.TRANSLUCENT;}
    }
    static final class Symbol extends Drawable {
        final String name;final Paint p=new Paint(Paint.ANTI_ALIAS_FLAG);final Path path=new Path();final int size;
        int opacity=255;Symbol(Context c,String name){this.name=name;size=dp(c,22);}
        @Override public int getIntrinsicWidth(){return size;}@Override public int getIntrinsicHeight(){return size;}
        @Override public boolean isStateful(){return true;}@Override protected boolean onStateChange(int[]s){invalidateSelf();return true;}
        @Override public void draw(Canvas c){
            boolean enabled=false,active=false;for(int s:getState()){if(s==android.R.attr.state_enabled)enabled=true;
                if(s==android.R.attr.state_checked||s==android.R.attr.state_focused||s==android.R.attr.state_pressed)active=true;}
            int mark=c.save();Rect b=getBounds();c.translate(b.left,b.top);c.scale(b.width()/24f,b.height()/24f);
            p.setColor(!enabled?0xff697184:active||name.equals("play")?LauncherAppearance.YELLOW:LauncherAppearance.BLUE);p.setAlpha(opacity);
            p.setStrokeWidth(1.7f);p.setStyle(Paint.Style.STROKE);p.setStrokeCap(Paint.Cap.ROUND);p.setStrokeJoin(Paint.Join.ROUND);path.reset();
            if(name.equals("play")){path.moveTo(7,4);path.lineTo(20,12);path.lineTo(7,20);path.close();p.setStyle(Paint.Style.FILL);c.drawPath(path,p);}
            else if(name.equals("display")){c.drawRect(2,4,22,17,p);c.drawLine(12,17,12,21,p);c.drawLine(7,21,17,21,p);}
            else if(name.equals("mods")){c.drawRect(2,3,18,13,p);c.drawRect(7,15,22,22,p);c.drawLine(5,6,10,6,p);}
            else if(name.equals("folder")){path.moveTo(2,20);path.lineTo(2,5);path.lineTo(9,5);path.lineTo(12,8);path.lineTo(22,8);path.lineTo(22,20);path.close();c.drawPath(path,p);c.drawLine(3,11,21,11,p);}
            else if(name.equals("updates")||name.equals("install")){c.drawLine(12,3,12,15,p);path.moveTo(7,10);path.lineTo(12,15);path.lineTo(17,10);c.drawPath(path,p);path.reset();path.moveTo(3,16);path.lineTo(3,21);path.lineTo(21,21);path.lineTo(21,16);c.drawPath(path,p);}
            else if(name.equals("back")){path.moveTo(11,5);path.lineTo(4,12);path.lineTo(11,19);c.drawPath(path,p);c.drawLine(4,12,21,12,p);}
            else if(name.equals("refresh")){c.drawArc(4,4,20,20,35,280,false,p);path.moveTo(16,3);path.lineTo(21,5);path.lineTo(21,10);c.drawPath(path,p);}
            else if(name.equals("help")){c.drawCircle(12,12,9,p);c.drawLine(12,10,12,17,p);p.setStyle(Paint.Style.FILL);c.drawCircle(12,7,1,p);}
            else {c.drawRect(4,4,20,20,p);c.drawLine(8,9,16,9,p);c.drawLine(8,13,16,13,p);c.drawLine(8,17,13,17,p);}
            c.restoreToCount(mark);
        }
        @Override public void setAlpha(int a){opacity=a;invalidateSelf();}@Override public void setColorFilter(ColorFilter f){p.setColorFilter(f);invalidateSelf();}
        @Override public int getOpacity(){return PixelFormat.TRANSLUCENT;}
    }
}
